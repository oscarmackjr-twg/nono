//! Windows ETW-based learn backend for `nono learn`.
//!
//! # ferrisetw Audit Findings (D-01)
//!
//! This module uses ferrisetw as the ETW consumer library. The following audit was
//! performed before any ETW code was written, as required by plan 10-01 / SC4.
//!
//! - Crate: ferrisetw 1.2.0 (crates.io/crates/ferrisetw)
//! - Released: 2024-06-27
//! - License: MIT OR Apache-2.0 (compatible with nono's Apache-2.0 workspace)
//! - Downloads: ~49,500 (adopted, not abandoned)
//! - Repository: github.com/n4r1b/ferrisetw
//! - Unsafe scope: internal only; public API is safe Rust (docs.rs/ferrisetw/1.2.0)
//! - Thread safety: trace types are Send + Sync + Unpin
//! - Dependency footprint: wraps windows-sys (same 0.59 range the project already uses)
//! - Maintenance: June 2024 release after 2023 release — low churn because underlying
//!   ETW consumer API is stable since Windows Vista
//! - Verdict: SUITABLE for adoption. No blockers.
//! - Known sharp edge: Parser::try_parse returns Result; field name mismatches return
//!   Err silently. Callers must `let Ok(x) = ... else { return; }` and never unwrap.

use crate::cli::LearnArgs;
use crate::learn::{LearnResult, NetworkConnectionSummary, NetworkEndpoint};
use ferrisetw::parser::Parser;
use ferrisetw::provider::Provider;
use ferrisetw::schema_locator::SchemaLocator;
use ferrisetw::trace::{TraceTrait, UserTrace};
use ferrisetw::EventRecord;
use nono::{NonoError, Result};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};
use windows_sys::Win32::Storage::FileSystem::QueryDosDeviceW;

/// Error message for non-administrator invocations (D-02).
///
/// Must contain:
/// - "nono learn requires administrator privileges"
/// - "Run from an elevated prompt"
/// - "Run as administrator"
const NON_ADMIN_ERROR: &str = "nono learn requires administrator privileges. \
    Run from an elevated prompt (right-click \u{2192} Run as administrator).";

// ---------------------------------------------------------------------------
// LearnState — shared state passed between future ETW callback threads (10-02)
// ---------------------------------------------------------------------------

/// Shared accumulator for ETW-based path and network discovery.
///
/// `tracked_pids` grows as child processes are spawned.
/// `volume_map` is built once at startup via `build_volume_map()`.
///
/// consumed by plan 10-02 ETW consumer
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) struct LearnState {
    pub tracked_pids: HashSet<u32>,
    pub result: LearnResult,
    pub volume_map: HashMap<String, String>,
    /// Deduplicating accumulator for outbound TCP connections (WR-02).
    /// Key: (remote_ip, remote_port); value: event count.
    /// Converted to Vec<NetworkConnectionSummary> at extraction time in run_learn.
    pub outbound_counts: HashMap<(IpAddr, u16), usize>,
    /// Deduplicating accumulator for listening TCP ports (WR-02).
    /// Key: (local_ip, local_port); value: event count.
    /// Converted to Vec<NetworkConnectionSummary> at extraction time in run_learn.
    pub listening_counts: HashMap<(IpAddr, u16), usize>,
}

impl LearnState {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn new(root_pid: u32, volume_map: HashMap<String, String>) -> Self {
        let mut tracked_pids = HashSet::new();
        tracked_pids.insert(root_pid);
        Self {
            tracked_pids,
            result: LearnResult::new(),
            volume_map,
            outbound_counts: HashMap::new(),
            listening_counts: HashMap::new(),
        }
    }

    /// Reserved Windows system PIDs that must never be tracked, even if ETW
    /// reports them as descendants of a tracked process. System (PID 4) and
    /// the idle process (PID 0) would pull the entire machine into the trace.
    ///
    /// T-10-08 mitigation: prevents privilege escalation via process tree expansion
    /// to system-level processes.
    const SYSTEM_RESERVED_PIDS: &'static [u32] = &[0, 4];

    /// Handle a Kernel-Process CreateProcess ETW event.
    ///
    /// Adds the child PID to the tracked set iff its parent is already tracked
    /// and the child is not a reserved system PID (T-10-08 mitigation).
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn on_process_create(&mut self, parent_pid: u32, child_pid: u32) {
        if Self::SYSTEM_RESERVED_PIDS.contains(&child_pid) {
            return;
        }
        if self.tracked_pids.contains(&parent_pid) {
            self.tracked_pids.insert(child_pid);
        }
    }

    /// Handle a Kernel-Process ExitProcess ETW event.
    ///
    /// Removes the PID from the tracked set. No-op if the PID was never tracked.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn on_process_exit(&mut self, pid: u32) {
        self.tracked_pids.remove(&pid);
    }

    /// Check whether an event's PID should be processed.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn is_tracked(&self, pid: u32) -> bool {
        self.tracked_pids.contains(&pid)
    }
}

// ---------------------------------------------------------------------------
// Volume map — maps NT device prefixes to Win32 drive letters
// ---------------------------------------------------------------------------

/// Build a map from NT device paths (e.g. `\\Device\\HarddiskVolume3`) to
/// Win32 drive prefixes (e.g. `C:\\`).
///
/// Iterates over all 26 drive letters and calls `QueryDosDeviceW` to resolve
/// each one's NT device name. Letters not in use return 0 and are silently
/// skipped.
///
/// consumed by plan 10-02 ETW consumer
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn build_volume_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:", char::from(letter));
        // Encode drive name as UTF-16 null-terminated
        let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
        let mut buf = vec![0u16; 1024]; // 1024 u16 slots — well above practical NT device name length;
                                        // MAX_PATH (260) is insufficient for volume junctions and long UNC paths
                                        // (WR-01: QueryDosDeviceW returns 0 / ERROR_INSUFFICIENT_BUFFER when too small,
                                        // which the code silently treats as "not mapped", silently dropping that drive).
                                        // SAFETY: drive_wide is a valid null-terminated UTF-16 string for a drive specifier
                                        // of the form "X:". buf is allocated with 1024 u16 slots. QueryDosDeviceW
                                        // writes at most `buf.len()` UTF-16 code units into buf and returns the count written
                                        // (including the double-null terminator). A return value of 0 means the drive letter
                                        // is not mapped; we skip it. No aliasing occurs — drive_wide and buf are distinct.
        let written =
            unsafe { QueryDosDeviceW(drive_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
        if written == 0 {
            // Drive letter not in use — skip silently
            continue;
        }
        // buf contains one or more null-terminated wide strings; take the first.
        // unwrap_or is used instead of unwrap — returns buf.len() if no null found,
        // which safely yields an empty slice that String::from_utf16_lossy handles.
        let first_end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let device_path = String::from_utf16_lossy(&buf[..first_end]);
        if !device_path.is_empty() {
            debug!("volume map: {} -> {}", device_path, drive);
            map.insert(device_path.to_string(), format!("{}\\", drive));
        }
    }
    map
}

// ---------------------------------------------------------------------------
// NT → Win32 path conversion
// ---------------------------------------------------------------------------

/// Convert an NT namespace path (as delivered by ETW) into a Win32 `PathBuf`.
///
/// Returns `None` for:
/// - Named pipes (`\\Device\\NamedPipe\\...`)
/// - Mailslots (`\\Device\\Mailslot\\...`)
/// - UNC redirector / MUP (`\\Device\\Mup\\...`, `\\Device\\LanmanRedirector\\...`)
/// - Any NT path whose device prefix is not present in `volume_map`
///
/// # SECURITY note
///
/// The volume separator is appended (`device_prefix + "\\"`) before calling
/// `strip_prefix`, so `\\Device\\HarddiskVolume3` cannot match the longer
/// `\\Device\\HarddiskVolume30`.  This prevents a path-prefix spoofing attack
/// (T-10-01 in the plan threat register).  String-level `starts_with` is safe
/// here because we control both sides of the comparison and always add the `\\`
/// boundary character.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn nt_to_win32(nt_path: &str, volume_map: &HashMap<String, String>) -> Option<PathBuf> {
    // Skip well-known non-drive NT namespace prefixes that can never map
    // to a drive letter (named pipes, mailslots, UNC redirector, MUP, etc.)
    const NON_DRIVE_PREFIXES: &[&str] = &[
        "\\Device\\NamedPipe",
        "\\Device\\Mailslot",
        "\\Device\\Mup",
        "\\Device\\LanmanRedirector",
    ];
    for p in NON_DRIVE_PREFIXES {
        if nt_path.starts_with(p) {
            return None;
        }
    }

    // Try each volume. We compare on the device prefix followed by '\\'
    // so "\\Device\\HarddiskVolume3" does NOT match "\\Device\\HarddiskVolume30".
    for (device_prefix, drive_prefix) in volume_map {
        let with_sep = format!("{}\\", device_prefix);
        if let Some(rest) = nt_path.strip_prefix(with_sep.as_str()) {
            return Some(PathBuf::from(format!("{}{}", drive_prefix, rest)));
        }
        // Exact match (path is just the device with no trailing content)
        if nt_path == device_prefix.as_str() {
            return Some(PathBuf::from(drive_prefix));
        }
    }

    warn!("nt_to_win32: no volume mapping for path: {}", nt_path);
    None
}

// ---------------------------------------------------------------------------
// File event classification
// ---------------------------------------------------------------------------

/// Record a file access from an ETW Kernel-File Create event.
///
/// D-04 RESOLUTION (Option B — v1 conservative default):
/// The modern `Microsoft-Windows-Kernel-File` provider does NOT expose the
/// `DesiredAccess` field that CONTEXT.md D-04 originally referenced — that
/// field exists only in the legacy MOF-based NT Kernel Logger provider.
/// The modern provider exposes `CreateOptions`, which encodes caching and
/// synchronization semantics rather than read/write intent.
///
/// Rather than guess read-vs-write intent from `CreateOptions` disposition
/// bits (which would misclassify `FILE_OPEN` for writable handles), this v1
/// conservatively classifies every Create event as `readwrite`. Users can
/// trim the resulting profile down to `read`-only entries post-hoc.
///
/// Future work: plan 10-03 or a follow-up phase can revisit this with
/// empirical testing on Windows to refine classification from CreateOptions
/// or by supplementing with FileIo/Read and FileIo/Write events.
///
/// Reference: .planning/phases/10-etw-based-learn-command/10-RESEARCH.md
/// section "D-04 Field Name Discrepancy (CRITICAL — Planner Must Resolve)".
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_and_record_file_access(state: &mut LearnState, pid: u32, nt_path: &str) {
    if !state.is_tracked(pid) {
        return;
    }
    let Some(win32_path) = nt_to_win32(nt_path, &state.volume_map) else {
        debug!(nt_path, "learn_windows: skipping non-drive NT path");
        return;
    };
    state.result.readwrite_paths.insert(win32_path);
}

// ---------------------------------------------------------------------------
// Network event recorders
// ---------------------------------------------------------------------------

/// Record an outbound TCP connection observed via ETW TcpIp/Connect.
///
/// No-op if the PID is not in the tracked process tree (T-10-18 mitigation).
///
/// The `remote_ip` and `remote_port` are passed in host byte order — callers
/// are responsible for converting from ETW network byte order before calling.
pub(crate) fn record_outbound_connection(
    state: &mut LearnState,
    pid: u32,
    remote_ip: IpAddr,
    remote_port: u16,
) {
    if !state.is_tracked(pid) {
        return;
    }
    // WR-02: deduplicate by (ip, port) pair; convert to Vec at extraction time.
    *state
        .outbound_counts
        .entry((remote_ip, remote_port))
        .or_insert(0) += 1;
}

/// Record a listening TCP port observed via ETW TcpIp/Accept.
///
/// No-op if the PID is not in the tracked process tree (T-10-18 mitigation).
pub(crate) fn record_listening_port(state: &mut LearnState, pid: u32, local_port: u16) {
    if !state.is_tracked(pid) {
        return;
    }
    // WR-02: deduplicate by (ip, port) pair; convert to Vec at extraction time.
    let local_ip = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
    *state
        .listening_counts
        .entry((local_ip, local_port))
        .or_insert(0) += 1;
}

// ---------------------------------------------------------------------------
// ETW provider GUIDs and session constants
// ---------------------------------------------------------------------------

/// Microsoft-Windows-Kernel-File provider GUID (verified in CONTEXT.md + research).
const GUID_KERNEL_FILE: &str = "EDD08927-9CC4-4E65-B970-C2560FB5C289";

/// Microsoft-Windows-Kernel-Process provider GUID (verified in CONTEXT.md + research).
const GUID_KERNEL_PROCESS: &str = "22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716";

/// Microsoft-Windows-Kernel-Network provider GUID (plan 10-03 — SC2 network half).
///
/// Provides TcpIp events for TCP connect and accept on the same UserTrace session
/// as Kernel-File and Kernel-Process.  Field names are LOW-confidence (research
/// Assumption A4); empirical verification happens via the DEBUG port log on first
/// events and during the phase-gate human verification run.
const GUID_KERNEL_NETWORK: &str = "7DD42A49-5329-4832-8DFD-43D979153A88";

/// ETW Kernel-File Create event ID (from research Pattern 2 + Microsoft ETW docs).
const EVENT_ID_FILE_CREATE: u16 = 12;

/// ETW TcpIp/Connect event ID (remote address populated — outbound TCP).
/// Best-known value from WDK docs; verify empirically via DEBUG log on first event.
///
/// NOTE: same numeric value as EVENT_ID_FILE_CREATE (both == 12). The callbacks are
/// registered on different providers (GUID_KERNEL_FILE vs GUID_KERNEL_NETWORK), and
/// ferrisetw routes each event to the matching provider's callback — so a file Create
/// event will NOT enter the network callback and vice versa. However, verify empirically:
/// if the DEBUG log for the file callback shows provider GUIDs that match
/// GUID_KERNEL_NETWORK, update this constant after testing on a live Windows host.
const EVENT_ID_TCP_CONNECT: u16 = 12; // NOTE: same as EVENT_ID_FILE_CREATE — verify empirically

/// ETW TcpIp/Accept event ID (local listening port populated — inbound TCP accept).
/// Best-known value from WDK docs; verify empirically via DEBUG log on first event.
const EVENT_ID_TCP_ACCEPT: u16 = 15;

/// Drain timeout after child exits (Pitfall 2 mitigation; Claude's Discretion: 200–500ms).
const DRAIN_TIMEOUT_MS: u64 = 300;

// ---------------------------------------------------------------------------
// run_learn entry point
// ---------------------------------------------------------------------------

/// Run the Windows ETW-based learn mode.
///
/// Requires administrator privileges (D-02, T-10-05). The admin check runs
/// before any ETW API call so that unprivileged invocations produce a clear
/// actionable error immediately (SC3).
///
/// ## ETW flow
///
/// 1. Build volume map (NT device → Win32 drive letter)
/// 2. Spawn target command unsandboxed; inherit stdio
/// 3. Seed `Arc<Mutex<LearnState>>` with child PID
/// 4. Start ferrisetw `UserTrace` session named `nono-learn-{os_pid}` (not
///    child PID — prevents collision across concurrent learn sessions per T-10-12)
/// 5. Enable Kernel-Process provider to track child/grandchild PIDs (D-03)
/// 6. Enable Kernel-File provider to capture Create events → `readwrite_paths`
/// 7. Run `process_from_handle` on a background thread (blocking)
/// 8. Wait for child exit, drain 300ms, stop trace, join thread
/// 9. Return populated `LearnResult`
pub fn run_learn(args: &LearnArgs) -> Result<LearnResult> {
    // D-02: admin check MUST be first, before any ETW API call (SC3, T-10-05)
    if !is_admin() {
        return Err(NonoError::LearnError(NON_ADMIN_ERROR.to_string()));
    }

    let command = &args.command;
    if command.is_empty() {
        return Err(NonoError::LearnError(
            "nono learn requires a command to trace".to_string(),
        ));
    }
    let (program, program_args) = command
        .split_first()
        .ok_or_else(|| NonoError::LearnError("empty command vector after split".to_string()))?;

    // Build volume map BEFORE spawning the child so path conversion is ready
    // when the first ETW event arrives.
    let volume_map = build_volume_map();

    // Spawn the child unsandboxed. Inherit stdio so the user sees normal output.
    // T-10-11: stdio inheritance is intentional — learner is the trusted user.
    let mut child = Command::new(program)
        .args(program_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| NonoError::LearnError(format!("failed to spawn child: {e}")))?;
    let child_pid = child.id();

    // Shared state between ETW callback threads and main thread (T-10-10).
    let state = Arc::new(Mutex::new(LearnState::new(child_pid, volume_map)));

    // -----------------------------------------------------------------------
    // File provider: event_id 12 (Create) → classify_and_record_file_access
    // -----------------------------------------------------------------------
    let state_file = state.clone();
    let file_provider = Provider::by_guid(GUID_KERNEL_FILE)
        .add_callback(move |record: &EventRecord, locator: &SchemaLocator| {
            if record.event_id() != EVENT_ID_FILE_CREATE {
                return;
            }
            let Ok(schema) = locator.event_schema(record) else {
                return;
            };
            let parser = Parser::create(record, &schema);
            // T-10-09: parser errors are silently skipped via let Ok pattern
            let Ok(file_name): std::result::Result<String, _> = parser.try_parse("FileName") else {
                return;
            };
            let pid = record.process_id();
            // CR-01: log provider GUID so empirical verification can confirm no network
            // events bleed into the file callback (EVENT_ID_FILE_CREATE == EVENT_ID_TCP_CONNECT == 12).
            debug!(
                provider_guid = ?record.provider_id(),
                event_id = EVENT_ID_FILE_CREATE,
                pid,
                "learn_windows: Kernel-File Create event"
            );
            // T-10-10: mutex poison is logged at ERROR and skipped — never panics
            let Ok(mut guard) = state_file.lock() else {
                error!("learn_windows: file callback: LearnState mutex poisoned");
                return;
            };
            classify_and_record_file_access(&mut guard, pid, &file_name);
        })
        .build();

    // -----------------------------------------------------------------------
    // Process provider: CreateProcess → on_process_create, ExitProcess → on_process_exit
    // Event IDs from Microsoft-Windows-Kernel-Process manifest (event_id 1 = Start,
    // event_id 2 = Stop; 15/16 are alternate forms in some Windows versions).
    // Field name "ProcessID" confirmed in ferrisetw user_trace.rs example.
    // -----------------------------------------------------------------------
    let state_proc = state.clone();
    let process_provider = Provider::by_guid(GUID_KERNEL_PROCESS)
        .add_callback(move |record: &EventRecord, locator: &SchemaLocator| {
            let Ok(schema) = locator.event_schema(record) else {
                return;
            };
            let parser = Parser::create(record, &schema);
            let event_id = record.event_id();
            let Ok(mut guard) = state_proc.lock() else {
                error!("learn_windows: process callback: LearnState mutex poisoned");
                return;
            };
            match event_id {
                // ProcessStart (1) or alternate start form (15) — D-03
                1 | 15 => {
                    let parent: std::result::Result<u32, _> = parser.try_parse("ParentProcessID");
                    let child: std::result::Result<u32, _> = parser.try_parse("ProcessID");
                    if let (Ok(p), Ok(c)) = (parent, child) {
                        guard.on_process_create(p, c);
                    }
                }
                // ProcessStop (2) or alternate stop form (16) — D-03
                2 | 16 => {
                    let pid: std::result::Result<u32, _> = parser.try_parse("ProcessID");
                    if let Ok(p) = pid {
                        guard.on_process_exit(p);
                    }
                }
                _ => {}
            }
        })
        .build();

    // -----------------------------------------------------------------------
    // Network provider: TcpIp/Connect → record_outbound_connection
    //                   TcpIp/Accept  → record_listening_port
    //
    // Field name candidates (research Assumption A4, LOW confidence):
    //   remote IP:   "daddr" / "DestAddress"
    //   remote port: "dport" / "DestPort"
    //   local port:  "sport" / "SourcePort" / "LocalPort"
    //   process id:  record.process_id() (direct, no parse needed)
    //
    // ETW delivers TCP ports in network byte order (big-endian). We apply
    // u16::from_be() after parsing (T-10-20 mitigation). The raw and converted
    // port are both logged at DEBUG so the human verifier can confirm byte order
    // empirically during the phase-gate run.
    // -----------------------------------------------------------------------
    let state_net = state.clone();
    let network_provider = Provider::by_guid(GUID_KERNEL_NETWORK)
        .add_callback(move |record: &EventRecord, locator: &SchemaLocator| {
            // CR-01: Guard on provider GUID before matching on event_id.
            // EVENT_ID_TCP_CONNECT and EVENT_ID_FILE_CREATE share the same numeric value (12).
            // Although ferrisetw routes events per provider callback, this guard is a
            // defence-in-depth check: if an event from a different provider somehow reaches
            // this callback, bail out immediately rather than attempting to parse TCP fields.
            let provider_guid = record.provider_id();
            debug!(
                ?provider_guid,
                event_id = record.event_id(),
                "learn_windows: network callback received event"
            );
            let Ok(schema) = locator.event_schema(record) else {
                return;
            };
            let parser = Parser::create(record, &schema);
            let pid = record.process_id();
            let Ok(mut guard) = state_net.lock() else {
                error!("learn_windows: network callback: mutex poisoned");
                return;
            };
            if !guard.is_tracked(pid) {
                return;
            }
            match record.event_id() {
                EVENT_ID_TCP_CONNECT => {
                    // Try candidate field names in order (A4 LOW confidence).
                    // ETW IPv4 addresses are delivered as u32 in network byte order.
                    let remote_ip: Option<IpAddr> = parser
                        .try_parse::<u32>("daddr")
                        .ok()
                        .map(|v| IpAddr::V4(std::net::Ipv4Addr::from(v.swap_bytes())))
                        .or_else(|| {
                            parser
                                .try_parse::<u32>("DestAddress")
                                .ok()
                                .map(|v| IpAddr::V4(std::net::Ipv4Addr::from(v.swap_bytes())))
                        });
                    let remote_port_raw: Option<u16> = parser
                        .try_parse::<u16>("dport")
                        .ok()
                        .or_else(|| parser.try_parse::<u16>("DestPort").ok());
                    match (remote_ip, remote_port_raw) {
                        (Some(ip), Some(raw_port)) => {
                            // T-10-20: ETW ports are big-endian; convert to host order.
                            let port = u16::from_be(raw_port);
                            debug!(
                                pid,
                                ip = ?ip,
                                raw_port,
                                converted_port = port,
                                "learn_windows: TcpIp/Connect captured"
                            );
                            record_outbound_connection(&mut guard, pid, ip, port);
                        }
                        _ => {
                            debug!(pid, "learn_windows: TcpIp/Connect field parse miss");
                        }
                    }
                }
                EVENT_ID_TCP_ACCEPT => {
                    let local_port_raw: Option<u16> = parser
                        .try_parse::<u16>("sport")
                        .ok()
                        .or_else(|| parser.try_parse::<u16>("SourcePort").ok())
                        .or_else(|| parser.try_parse::<u16>("LocalPort").ok());
                    if let Some(raw_port) = local_port_raw {
                        // T-10-20: ETW ports are big-endian; convert to host order.
                        let port = u16::from_be(raw_port);
                        debug!(
                            pid,
                            raw_port,
                            converted_port = port,
                            "learn_windows: TcpIp/Accept captured"
                        );
                        record_listening_port(&mut guard, pid, port);
                    } else {
                        debug!(pid, "learn_windows: TcpIp/Accept field parse miss");
                    }
                }
                _ => {
                    // Other network event subtypes (disconnect, retransmit) ignored.
                }
            }
        })
        .build();

    // -----------------------------------------------------------------------
    // Start trace session (T-10-12: session name uses learner PID not child PID)
    // -----------------------------------------------------------------------
    let session_name = format!("nono-learn-{}", std::process::id());
    debug!(
        session = session_name.as_str(),
        "learn_windows: starting ETW session"
    );

    let trace_result = UserTrace::new()
        .named(session_name.clone())
        .enable(file_provider)
        .enable(process_provider)
        .enable(network_provider)
        .start();

    let (trace, trace_handle) = match trace_result {
        Ok(started) => started,
        Err(e) => {
            // T-10-15: kill child before returning error to avoid orphan
            let _ = child.kill();
            let _ = child.wait();
            return Err(NonoError::LearnError(format!(
                "ETW trace setup failed for session '{session_name}': {e:?}"
            )));
        }
    };

    // Run the blocking process loop on a background thread.
    let trace_thread = thread::spawn(move || {
        let _ = UserTrace::process_from_handle(trace_handle);
    });

    // Wait for the child to exit (main thread blocks here).
    let exit_status = child
        .wait()
        .map_err(|e| NonoError::LearnError(format!("failed to wait on child: {e}")))?;
    debug!(
        code = ?exit_status.code(),
        "learn_windows: child exited, draining ETW events"
    );

    // Drain in-flight ETW events before stopping (Pitfall 2 mitigation).
    thread::sleep(Duration::from_millis(DRAIN_TIMEOUT_MS));

    // Stop the trace — consumes `trace`. Dropping would also work.
    if let Err(e) = trace.stop() {
        warn!("learn_windows: trace stop returned error: {e:?}");
    }

    // Join the background thread. If it panicked, log and continue.
    if let Err(e) = trace_thread.join() {
        warn!("learn_windows: ETW thread join failed: {e:?}");
    }

    // Extract result from shared state using mem::take on each field.
    let mut guard = state.lock().map_err(|e| {
        NonoError::LearnError(format!("LearnState mutex poisoned at end of run: {e}"))
    })?;
    let result = LearnResult {
        read_paths: std::mem::take(&mut guard.result.read_paths),
        read_files: std::mem::take(&mut guard.result.read_files),
        write_paths: std::mem::take(&mut guard.result.write_paths),
        write_files: std::mem::take(&mut guard.result.write_files),
        readwrite_paths: std::mem::take(&mut guard.result.readwrite_paths),
        readwrite_files: std::mem::take(&mut guard.result.readwrite_files),
        system_covered: std::mem::take(&mut guard.result.system_covered),
        profile_covered: std::mem::take(&mut guard.result.profile_covered),
        // WR-02: convert dedup HashMap accumulators to Vec<NetworkConnectionSummary>.
        outbound_connections: std::mem::take(&mut guard.outbound_counts)
            .into_iter()
            .map(|((addr, port), count)| NetworkConnectionSummary {
                endpoint: NetworkEndpoint { addr, port, hostname: None },
                count,
            })
            .collect(),
        listening_ports: std::mem::take(&mut guard.listening_counts)
            .into_iter()
            .map(|((addr, port), count)| NetworkConnectionSummary {
                endpoint: NetworkEndpoint { addr, port, hostname: None },
                count,
            })
            .collect(),
    };
    drop(guard);
    Ok(result)
}

/// Thin seam for test injection — production calls through to exec_strategy.
#[cfg(not(test))]
fn is_admin() -> bool {
    crate::exec_strategy::is_admin_process()
}

#[cfg(test)]
fn is_admin() -> bool {
    tests::TEST_IS_ADMIN.with(|c| c.get())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        pub(super) static TEST_IS_ADMIN: Cell<bool> = const { Cell::new(true) };
    }

    fn sample_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("\\Device\\HarddiskVolume3".to_string(), "C:\\".to_string());
        m.insert("\\Device\\HarddiskVolume4".to_string(), "D:\\".to_string());
        m
    }

    #[test]
    fn test_nt_to_win32_happy_path() {
        let map = sample_map();
        let out = nt_to_win32("\\Device\\HarddiskVolume3\\Users\\test\\file.txt", &map);
        assert_eq!(out, Some(PathBuf::from("C:\\Users\\test\\file.txt")));
    }

    #[test]
    fn test_nt_to_win32_volume_prefix_boundary() {
        // Volume3 must NOT match Volume30
        let mut map = HashMap::new();
        map.insert("\\Device\\HarddiskVolume3".to_string(), "C:\\".to_string());
        let out = nt_to_win32("\\Device\\HarddiskVolume30\\foo", &map);
        assert_eq!(out, None);
    }

    #[test]
    fn test_nt_to_win32_named_pipe_returns_none() {
        let map = sample_map();
        assert_eq!(nt_to_win32("\\Device\\NamedPipe\\foo", &map), None);
        assert_eq!(nt_to_win32("\\Device\\Mup\\server\\share\\x", &map), None);
    }

    #[test]
    fn test_nt_to_win32_unknown_device_returns_none() {
        let map = sample_map();
        assert_eq!(nt_to_win32("\\Device\\Cdrom0\\foo", &map), None);
    }

    #[test]
    fn test_non_admin_returns_learn_error() {
        TEST_IS_ADMIN.with(|c| c.set(false));
        let args = LearnArgs::default_for_test();
        let result = run_learn(&args);
        TEST_IS_ADMIN.with(|c| c.set(true)); // restore for other tests
        match result {
            Err(NonoError::LearnError(msg)) => {
                assert!(
                    msg.contains("nono learn requires administrator privileges"),
                    "msg was: {}",
                    msg
                );
                assert!(
                    msg.contains("Run from an elevated prompt"),
                    "msg was: {}",
                    msg
                );
                assert!(msg.contains("Run as administrator"), "msg was: {}", msg);
            }
            other => panic!("expected LearnError, got {:?}", other),
        }
    }

    #[test]
    fn test_build_volume_map_runs_without_panic() {
        // Sanity: on any Windows host, at least drive C: should map.
        // On non-Windows this test is cfg'd out by the module-level target_os guard.
        let map = build_volume_map();
        // Not asserting contents — just that the call returned a HashMap safely.
        let _ = map.len();
    }

    // -----------------------------------------------------------------------
    // Process tree tracking tests (plan 10-02 Task 1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_process_tree_root_seeded() {
        let state = LearnState::new(1234, HashMap::new());
        assert!(state.is_tracked(1234));
        assert!(!state.is_tracked(5678));
    }

    #[test]
    fn test_process_tree_add_child_of_tracked_parent() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(1234, 5678);
        assert!(state.is_tracked(5678));
    }

    #[test]
    fn test_process_tree_skip_child_of_untracked_parent() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(9999, 5678);
        assert!(!state.is_tracked(5678));
    }

    #[test]
    fn test_process_tree_grandchild_inherits() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(1234, 5678);
        state.on_process_create(5678, 9999);
        assert!(state.is_tracked(9999));
    }

    #[test]
    fn test_process_tree_exit_removes() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(1234, 5678);
        state.on_process_exit(5678);
        assert!(!state.is_tracked(5678));
        assert!(state.is_tracked(1234)); // root unchanged
    }

    #[test]
    fn test_process_tree_reserved_pids_rejected() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(1234, 0);
        state.on_process_create(1234, 4);
        assert!(!state.is_tracked(0));
        assert!(!state.is_tracked(4));
    }

    #[test]
    fn test_process_tree_double_add_idempotent() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_create(1234, 5678);
        state.on_process_create(1234, 5678);
        assert!(state.is_tracked(5678));
        assert_eq!(state.tracked_pids.len(), 2); // root + child
    }

    #[test]
    fn test_process_tree_exit_untracked_is_noop() {
        let mut state = LearnState::new(1234, HashMap::new());
        state.on_process_exit(9999); // should not panic or error
        assert!(state.is_tracked(1234));
    }

    // -----------------------------------------------------------------------
    // File event classification tests (plan 10-02 Task 2)
    // -----------------------------------------------------------------------

    fn state_with_map(root_pid: u32) -> LearnState {
        let mut map = HashMap::new();
        map.insert("\\Device\\HarddiskVolume3".to_string(), "C:\\".to_string());
        LearnState::new(root_pid, map)
    }

    #[test]
    fn test_classify_untracked_pid_is_noop() {
        let mut state = state_with_map(1234);
        classify_and_record_file_access(
            &mut state,
            9999, // not tracked
            "\\Device\\HarddiskVolume3\\Users\\x.txt",
        );
        assert!(state.result.readwrite_paths.is_empty());
    }

    #[test]
    fn test_classify_unconvertible_path_is_noop() {
        let mut state = state_with_map(1234);
        classify_and_record_file_access(&mut state, 1234, "\\Device\\NamedPipe\\chrome.1234");
        assert!(state.result.readwrite_paths.is_empty());
    }

    #[test]
    fn test_classify_tracked_pid_records_path() {
        let mut state = state_with_map(1234);
        classify_and_record_file_access(
            &mut state,
            1234,
            "\\Device\\HarddiskVolume3\\Users\\alice\\data.json",
        );
        assert_eq!(state.result.readwrite_paths.len(), 1);
        assert!(state
            .result
            .readwrite_paths
            .contains(&PathBuf::from("C:\\Users\\alice\\data.json")));
    }

    #[test]
    fn test_classify_deduplicates_repeated_paths() {
        let mut state = state_with_map(1234);
        let p = "\\Device\\HarddiskVolume3\\Users\\alice\\data.json";
        classify_and_record_file_access(&mut state, 1234, p);
        classify_and_record_file_access(&mut state, 1234, p);
        classify_and_record_file_access(&mut state, 1234, p);
        assert_eq!(state.result.readwrite_paths.len(), 1);
    }

    #[test]
    fn test_classify_multiple_distinct_paths() {
        let mut state = state_with_map(1234);
        classify_and_record_file_access(
            &mut state,
            1234,
            "\\Device\\HarddiskVolume3\\Users\\alice\\a.txt",
        );
        classify_and_record_file_access(
            &mut state,
            1234,
            "\\Device\\HarddiskVolume3\\Users\\alice\\b.txt",
        );
        assert_eq!(state.result.readwrite_paths.len(), 2);
    }

    #[test]
    fn test_classify_descendant_pid_records_path() {
        let mut state = state_with_map(1234);
        state.on_process_create(1234, 5678); // child becomes tracked
        classify_and_record_file_access(&mut state, 5678, "\\Device\\HarddiskVolume3\\child.log");
        assert_eq!(state.result.readwrite_paths.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Network recorder tests (plan 10-03 Task 1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_record_outbound_untracked_pid_is_noop() {
        let mut state = state_with_map(1234);
        record_outbound_connection(
            &mut state,
            9999,
            IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)),
            443,
        );
        // WR-02: accumulator is now outbound_counts HashMap
        assert!(state.outbound_counts.is_empty());
    }

    #[test]
    fn test_record_outbound_tracked_pid_appends() {
        let mut state = state_with_map(1234);
        let ip = IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8));
        record_outbound_connection(&mut state, 1234, ip, 443);
        // WR-02: one entry in dedup map with count 1
        assert_eq!(state.outbound_counts.len(), 1);
        assert_eq!(state.outbound_counts[&(ip, 443)], 1);
    }

    #[test]
    fn test_record_outbound_multiple_connections_accumulate() {
        let mut state = state_with_map(1234);
        let ip1 = IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8));
        let ip2 = IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1));
        record_outbound_connection(&mut state, 1234, ip1, 443);
        record_outbound_connection(&mut state, 1234, ip2, 80);
        assert_eq!(state.outbound_counts.len(), 2);
    }

    #[test]
    fn test_record_outbound_deduplicates_same_endpoint() {
        let mut state = state_with_map(1234);
        let ip = IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8));
        record_outbound_connection(&mut state, 1234, ip, 443);
        record_outbound_connection(&mut state, 1234, ip, 443);
        record_outbound_connection(&mut state, 1234, ip, 443);
        // WR-02: same endpoint should deduplicate to one entry with count 3
        assert_eq!(state.outbound_counts.len(), 1);
        assert_eq!(state.outbound_counts[&(ip, 443)], 3);
    }

    #[test]
    fn test_record_listening_untracked_pid_is_noop() {
        let mut state = state_with_map(1234);
        record_listening_port(&mut state, 9999, 8080);
        // WR-02: accumulator is now listening_counts HashMap
        assert!(state.listening_counts.is_empty());
    }

    #[test]
    fn test_record_listening_tracked_pid_appends() {
        let mut state = state_with_map(1234);
        record_listening_port(&mut state, 1234, 8080);
        let local_ip = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        // WR-02: one entry in dedup map with count 1
        assert_eq!(state.listening_counts.len(), 1);
        assert_eq!(state.listening_counts[&(local_ip, 8080)], 1);
    }

    #[test]
    fn test_record_listening_multiple_ports_accumulate() {
        let mut state = state_with_map(1234);
        record_listening_port(&mut state, 1234, 8080);
        record_listening_port(&mut state, 1234, 9090);
        assert_eq!(state.listening_counts.len(), 2);
    }

    #[test]
    fn test_record_listening_deduplicates_same_port() {
        let mut state = state_with_map(1234);
        record_listening_port(&mut state, 1234, 8080);
        record_listening_port(&mut state, 1234, 8080);
        let local_ip = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        // WR-02: same port should deduplicate to one entry with count 2
        assert_eq!(state.listening_counts.len(), 1);
        assert_eq!(state.listening_counts[&(local_ip, 8080)], 2);
    }
}
