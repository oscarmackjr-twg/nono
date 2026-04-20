//! Windows named-pipe IPC for supervisor-child communication.
//!
//! Windows does not support Unix domain sockets or `SCM_RIGHTS`. The initial
//! supervisor scaffold uses a per-session duplex named pipe as the control
//! channel. Approved resource transfer uses explicit handle brokering metadata
//! plus `DuplicateHandle` into the child process.

use crate::error::{NonoError, Result};
use crate::supervisor::types::{
    PipeDirection, ResourceGrant, SocketRole, SupervisorMessage, SupervisorResponse,
};
use getrandom::fill as random_fill;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{
    DuplicateHandle, LocalFree, DUPLICATE_SAME_ACCESS, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY,
    ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Networking::WinSock::{
    WSADuplicateSocketW, INVALID_SOCKET, SOCKET, WSAPROTOCOL_INFOW,
};
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, CreatePipe, DisconnectNamedPipe,
    GetNamedPipeServerProcessId, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetProcessId};

/// SDDL revision used by `ConvertStringSecurityDescriptorToSecurityDescriptorW`.
const SDDL_REVISION_1: u32 = 1;

/// SDDL string for the capability pipe. Grants full access to SYSTEM, Built-in
/// Administrators, and the owner; adds a mandatory integrity SACL allowing
/// Low Integrity processes to write (`NW`) to the pipe.
///
/// This constant is used verbatim when the caller does NOT supply a per-session
/// restricting SID (in-process tests, Phase 11 / Phase 18 AIPC pipe callers).
/// When a per-session SID *is* supplied via
/// [`SupervisorSocket::bind_low_integrity_with_session_sid`], an additional
/// ACE granting `FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE`
/// (mask 0x120089) to that SID is appended before the SACL — necessary so
/// Phase 13's `CreateRestrictedToken(WRITE_RESTRICTED, ..., &session_sid, ...)`
/// child passes the second-pass DACL check against the restricting SID set
/// when it opens the pipe with `GENERIC_READ | GENERIC_WRITE`.
const CAPABILITY_PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)";

/// Access mask granted to the per-session restricting SID when one is supplied.
///
/// `FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE` = `0x0012019F`:
///   * `FILE_GENERIC_READ`  = `0x00120089` (`STANDARD_RIGHTS_READ`  | `FILE_READ_DATA`  | `FILE_READ_ATTRIBUTES`  | `FILE_READ_EA`  | `SYNCHRONIZE`)
///   * `FILE_GENERIC_WRITE` = `0x00120116` (`STANDARD_RIGHTS_WRITE` | `FILE_WRITE_DATA` | `FILE_WRITE_ATTRIBUTES` | `FILE_WRITE_EA` | `FILE_APPEND_DATA` | `SYNCHRONIZE`)
///   * union (including `SYNCHRONIZE` = `0x00100000`) = `0x0012019F`
///
/// **Critical: object-specific rights, NOT generic rights.**
///
/// SDDL mnemonic forms `GR` / `GW` / `GRGW` are stored verbatim in the DACL's
/// ACE mask (as `GENERIC_READ` = `0x80000000` and `GENERIC_WRITE` =
/// `0x40000000`). They are NOT expanded into object-specific rights at
/// SD-conversion time — Windows only applies the object's generic mapping at
/// access-check time. The mismatch between the child's access request
/// (`CreateFileW(GENERIC_READ | GENERIC_WRITE)` → mapped to
/// `FILE_GENERIC_READ | FILE_GENERIC_WRITE` = `0x12019F`) and the DACL's
/// stored generic mask (`0xC0000000`) causes the access check to fail when
/// the second DACL pass (against restricting SIDs) runs, because
/// `0xC0000000 & 0x12019F == 0`.
///
/// The regression test
/// `capability_pipe_admits_restricted_token_child_with_session_sid` walks the
/// parsed DACL and asserts this ACE carries exactly `0x12019F` — that
/// assertion would fail on any `G*` mnemonic. Auditors should read
/// `.planning/debug/supervisor-pipe-access-denied.md` for the full analysis.
const CAPABILITY_PIPE_RESTRICTING_SID_MASK: &str = "0x0012019F";

/// Maximum accepted length for a session SID string when embedding in SDDL.
///
/// The synthetic SIDs produced by `generate_session_sid` in
/// `nono-cli/src/exec_strategy_windows/restricted_token.rs` are
/// `S-1-5-117-<u32>-<u32>-<u32>-<u32>` — well under 64 characters. 128 is a
/// generous ceiling that still rejects pathological input (SDDL injection
/// defense-in-depth alongside the character-class filter).
const SESSION_SID_MAX_LEN: usize = 128;

/// Length prefix size: 4 bytes (u32 big-endian)
const LENGTH_PREFIX_SIZE: usize = 4;

/// Maximum message size: 64 KiB (prevents memory exhaustion from malicious messages)
const MAX_MESSAGE_SIZE: u32 = 64 * 1024;

/// Default wait for pipe availability during startup.
const PIPE_CONNECT_TIMEOUT_MS: u32 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PipeRendezvousInfo {
    pipe_name: String,
    server_pid: u32,
}

/// A Windows named pipe used for supervisor IPC.
#[derive(Debug)]
pub struct SupervisorSocket {
    reader: File,
    writer: File,
    transport_name: String,
    disconnect_on_drop: bool,
    cleanup_rendezvous_path: Option<PathBuf>,
}

/// Windows process target for brokered handle duplication.
#[derive(Debug, Clone, Copy)]
pub struct BrokerTargetProcess {
    handle: HANDLE,
}

impl SupervisorSocket {
    /// Create a connected Windows pipe pair for supervisor-child IPC.
    #[must_use = "both pipe ends must be used"]
    pub fn pair() -> Result<(Self, Self)> {
        // `pair()` is a local already-connected helper used by tests and
        // in-process setup. The production Windows supervisor transport still
        // uses duplex named pipes through `bind()` / `connect()`.
        let (parent_reader, child_writer) = create_anonymous_pipe()?;
        let (child_reader, parent_writer) = create_anonymous_pipe()?;
        let transport_name = unique_pair_name()?;

        Ok((
            Self {
                reader: parent_reader,
                writer: parent_writer,
                transport_name: transport_name.clone(),
                disconnect_on_drop: false,
                cleanup_rendezvous_path: None,
            },
            Self {
                reader: child_reader,
                writer: child_writer,
                transport_name,
                disconnect_on_drop: false,
                cleanup_rendezvous_path: None,
            },
        ))
    }

    /// Bind a named pipe for the provided rendezvous path and wait for a client.
    pub fn bind(path: &Path) -> Result<Self> {
        Self::bind_impl(path, false, None)
    }

    /// Bind a named pipe that accepts connections from Low Integrity processes.
    ///
    /// This variant attaches SDDL `S:(ML;;NW;;;LW)` so that a sandboxed child
    /// running with Low Mandatory Integrity (Windows Vista+ MIC) can write to
    /// the pipe. All other access controls (DACL restricting to SYSTEM,
    /// Built-in Administrators, and the owner) are preserved. On SDDL
    /// conversion failure this function fails secure — it does NOT fall back
    /// to a null security descriptor.
    ///
    /// Back-compat thin wrapper over
    /// [`SupervisorSocket::bind_low_integrity_with_session_sid`] that forwards
    /// `None` for the per-session restricting SID. The resulting SDDL is
    /// byte-identical to the pre-fix [`CAPABILITY_PIPE_SDDL`] constant, so
    /// in-process tests and Phase 18 AIPC pipe callers are unaffected.
    pub fn bind_low_integrity(path: &Path) -> Result<Self> {
        Self::bind_low_integrity_with_session_sid(path, None)
    }

    /// Bind a Low-Integrity-accessible named pipe and, when a per-session
    /// restricting SID is provided, attach an additional DACL ACE granting
    /// `FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE` to that SID.
    ///
    /// Phase 13 launches sandboxed children with
    /// `CreateRestrictedToken(..., WRITE_RESTRICTED, ..., 1, &session_sid, ...)`
    /// where `session_sid` is the synthetic per-session
    /// `S-1-5-117-<guid>`. For any WRITE access (including `GENERIC_WRITE`
    /// on `CreateFileW`) Windows performs the DACL check twice — once against
    /// the child's normal SIDs, once against the restricting SID set. The
    /// capability pipe's baseline DACL grants access only to SYSTEM,
    /// BUILTIN\Administrators, and OWNER_RIGHTS; the restricting SID is
    /// absent from every ACL on the system, so the second pass fails and
    /// Windows returns `ERROR_ACCESS_DENIED` on the child's `CreateFileW`.
    /// Passing `Some(&session_sid)` here appends
    /// `(A;;0x120089;;;<session_sid>)` to the DACL so that second-pass check
    /// succeeds.
    ///
    /// # Security (SDDL injection defense)
    ///
    /// The SID string is embedded into a dynamically constructed SDDL, which
    /// makes it a direct injection surface. Before embedding, the SID is
    /// validated via [`validate_session_sid_for_sddl`]: must begin with
    /// `S-1-`, contain only ASCII digits and hyphens after the prefix, and
    /// be no longer than [`SESSION_SID_MAX_LEN`]. Malformed input triggers a
    /// fail-closed `NonoError::SandboxInit` error — this function does NOT
    /// silently fall back to the no-SID SDDL path.
    pub fn bind_low_integrity_with_session_sid(
        path: &Path,
        session_sid: Option<&str>,
    ) -> Result<Self> {
        Self::bind_impl(path, true, session_sid)
    }

    fn bind_impl(path: &Path, low_integrity: bool, session_sid: Option<&str>) -> Result<Self> {
        let (pipe_name, cleanup_rendezvous_path) = prepare_bind_pipe_name(path)?;
        let server_handle = if low_integrity {
            create_low_integrity_named_pipe(&pipe_name, session_sid)?
        } else {
            // Non-low-integrity path has no restricting-SID DACL ACE — the
            // caller does not expect Low-IL or restricted-token children.
            debug_assert!(
                session_sid.is_none(),
                "session_sid is only meaningful on the low_integrity path"
            );
            create_named_pipe(&pipe_name, false)?
        };
        if let Err(err) = write_pipe_rendezvous(path, &pipe_name) {
            // SAFETY: `server_handle` is an owned handle created above and must
            // be reclaimed if rendezvous publication fails before conversion.
            drop(unsafe { OwnedHandle::from_raw_handle(server_handle) });
            return Err(err);
        }
        let server_file = finalize_server_connection(server_handle, &pipe_name)?;
        Ok(Self {
            reader: server_file
                .try_clone()
                .map_err(|e| NonoError::SandboxInit(format!("Failed to clone pipe handle: {e}")))?,
            writer: server_file,
            transport_name: pipe_name,
            disconnect_on_drop: true,
            cleanup_rendezvous_path,
        })
    }

    /// Connect to a named pipe published for the provided rendezvous path.
    pub fn connect(path: &Path) -> Result<Self> {
        let rendezvous = resolve_connect_pipe_name(path)?;
        let file = connect_named_pipe(&rendezvous)?;
        Ok(Self {
            reader: file
                .try_clone()
                .map_err(|e| NonoError::SandboxInit(format!("Failed to clone pipe handle: {e}")))?,
            writer: file,
            transport_name: rendezvous.pipe_name,
            disconnect_on_drop: false,
            cleanup_rendezvous_path: None,
        })
    }

    /// Send a message from child to supervisor.
    pub fn send_message(&mut self, msg: &SupervisorMessage) -> Result<()> {
        let payload = serde_json::to_vec(msg).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to serialize supervisor message: {e}"))
        })?;
        self.write_frame(&payload)
    }

    /// Receive a message from child (supervisor side).
    pub fn recv_message(&mut self) -> Result<SupervisorMessage> {
        let payload = self.read_frame()?;
        serde_json::from_slice(&payload).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to deserialize supervisor message: {e}"))
        })
    }

    /// Send a response from supervisor to child.
    pub fn send_response(&mut self, resp: &SupervisorResponse) -> Result<()> {
        let payload = serde_json::to_vec(resp).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to serialize supervisor response: {e}"))
        })?;
        self.write_frame(&payload)
    }

    /// Receive a response from supervisor (child side).
    pub fn recv_response(&mut self) -> Result<SupervisorResponse> {
        let payload = self.read_frame()?;
        serde_json::from_slice(&payload).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to deserialize supervisor response: {e}"))
        })
    }

    /// Returns the Windows transport name used by this connection.
    #[must_use]
    pub fn transport_name(&self) -> &str {
        &self.transport_name
    }

    fn write_frame(&mut self, payload: &[u8]) -> Result<()> {
        let len = payload.len();
        if len > MAX_MESSAGE_SIZE as usize {
            return Err(NonoError::SandboxInit(format!(
                "Supervisor message too large: {len} bytes (max: {MAX_MESSAGE_SIZE})"
            )));
        }

        let len_bytes = (len as u32).to_be_bytes();
        self.writer
            .write_all(&len_bytes)
            .map_err(|e| NonoError::SandboxInit(format!("Failed to write message length: {e}")))?;
        self.writer
            .write_all(payload)
            .map_err(|e| NonoError::SandboxInit(format!("Failed to write message payload: {e}")))?;
        self.writer
            .flush()
            .map_err(|e| NonoError::SandboxInit(format!("Failed to flush pipe payload: {e}")))?;
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Vec<u8>> {
        let mut len_bytes = [0u8; LENGTH_PREFIX_SIZE];
        self.reader
            .read_exact(&mut len_bytes)
            .map_err(|e| NonoError::SandboxInit(format!("Failed to read message length: {e}")))?;

        let len = u32::from_be_bytes(len_bytes);
        if len > MAX_MESSAGE_SIZE {
            return Err(NonoError::SandboxInit(format!(
                "Supervisor message too large: {len} bytes (max: {MAX_MESSAGE_SIZE})"
            )));
        }

        let mut payload = vec![0u8; len as usize];
        self.reader
            .read_exact(&mut payload)
            .map_err(|e| NonoError::SandboxInit(format!("Failed to read message payload: {e}")))?;
        Ok(payload)
    }
}

impl BrokerTargetProcess {
    /// Use the current process as the duplication target.
    #[must_use]
    pub fn current() -> Self {
        Self {
            handle: unsafe {
                // SAFETY: `GetCurrentProcess` returns a valid pseudo-handle for
                // the current process and requires no explicit cleanup.
                GetCurrentProcess()
            },
        }
    }

    /// Construct a target wrapper from an existing live process handle.
    ///
    /// # Safety
    ///
    /// The caller must ensure the handle refers to a live process and remains
    /// valid for the duration of the brokering call.
    #[must_use]
    pub unsafe fn from_raw_handle(handle: HANDLE) -> Self {
        Self { handle }
    }

    fn raw(self) -> HANDLE {
        self.handle
    }

    /// Return the OS-level PID for this target process.
    ///
    /// For [`BrokerTargetProcess::current()`] the underlying handle is the
    /// pseudo-handle returned by `GetCurrentProcess`; `GetProcessId` resolves
    /// it to the real PID. For wrapped process handles, `GetProcessId` returns
    /// the target's PID. Returns 0 only when the underlying call fails (an
    /// invalid handle), which the [`broker_target_pid`] free function falls
    /// back to the current PID for safety.
    ///
    /// Phase 18 AIPC-01 Plan 18-02: required by the Socket broker so
    /// `WSADuplicateSocketW(socket, target_pid, &mut info)` can bind the
    /// resulting `WSAPROTOCOL_INFOW` blob to the correct process.
    #[must_use]
    pub fn pid(self) -> u32 {
        // SAFETY: `self.handle` is either the GetCurrentProcess pseudo-handle
        // (always valid) or a process HANDLE wrapped via from_raw_handle whose
        // caller asserted liveness for the broker call. GetProcessId reads the
        // PID from the kernel object the handle refers to and returns 0 on
        // failure (no UB on bad handle — it just returns 0).
        unsafe { GetProcessId(self.handle) }
    }
}

/// Resolve the target PID for a brokered Socket request.
///
/// Wraps [`BrokerTargetProcess::pid`] with a fail-safe fallback to the current
/// process's PID when `GetProcessId` returns 0 (e.g. when target is the
/// pseudo-handle in test code). The Socket broker uses this in lieu of a
/// caller-supplied PID for `WSADuplicateSocketW`.
#[must_use]
pub fn broker_target_pid(target: &BrokerTargetProcess) -> u32 {
    let pid = target.pid();
    if pid == 0 {
        // SAFETY: GetCurrentProcessId is a leaf-safe Win32 call with no
        // arguments; always returns the calling process's PID.
        unsafe { GetCurrentProcessId() }
    } else {
        pid
    }
}

/// Duplicate an opened file handle into the target process and describe it in
/// the supervisor response contract.
pub fn broker_file_handle_to_process(
    file: &File,
    target_process: BrokerTargetProcess,
    access: crate::capability::AccessMode,
) -> Result<ResourceGrant> {
    let mut duplicated: HANDLE = std::ptr::null_mut();
    let source_handle = file.as_raw_handle() as HANDLE;

    // SAFETY: `source_handle` comes from a live `File`, the current process is
    // the source process, `target_process` was wrapped by the caller from a
    // live process handle, and `duplicated` points to writable storage.
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            source_handle,
            target_process.raw(),
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "Failed to duplicate Windows handle into supervised child: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok(ResourceGrant::duplicated_windows_file_handle(
        duplicated as usize as u64,
        access,
    ))
}

/// Duplicate a Windows event kernel-object handle into the target process
/// with the caller-validated access `mask`.
///
/// Per CONTEXT.md D-10 (Phase 18 AIPC-01): the supervisor closes its source
/// `handle` AFTER `send_response` returns; this function does NOT close the
/// source handle. The child owns the duplicated handle and is responsible
/// for closing it.
///
/// The mask is pre-validated by the caller against the per-session AIPC
/// allowlist via `policy::mask_is_allowed`; this function trusts the mask
/// and performs the FFI call.
///
/// The function is `safe` because the caller has already validated `handle`
/// is a live event kernel-object handle owned by this process (the supervisor
/// opened it via `CreateEventW`); `DuplicateHandle` itself is the only FFI
/// the function performs and the unsafe contract for that call is documented
/// inside.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn broker_event_to_process(
    handle: HANDLE,
    target_process: BrokerTargetProcess,
    mask: u32,
) -> Result<ResourceGrant> {
    let mut duplicated: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live event kernel-object handle owned by the
    // supervisor. `target_process.raw()` was wrapped from a live process
    // handle. `duplicated` points to writable storage. `mask` was validated
    // by the caller against the per-session allowlist via
    // `policy::mask_is_allowed` before this call. `dwOptions = 0` (NOT
    // DUPLICATE_SAME_ACCESS) because we MAP DOWN — the supervisor source
    // may have full EVENT_ALL_ACCESS but the child only gets the validated
    // subset (T-18-01-11 mitigation).
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            target_process.raw(),
            &mut duplicated,
            mask,
            0,
            0,
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (event, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ResourceGrant::duplicated_windows_event_handle(
        duplicated as usize as u64,
        mask,
    ))
}

/// Duplicate a Windows mutex kernel-object handle into the target process
/// with the caller-validated access `mask`.
///
/// Per CONTEXT.md D-10 (Phase 18 AIPC-01): the supervisor closes its source
/// `handle` AFTER `send_response` returns; this function does NOT close the
/// source handle. The child owns the duplicated handle and is responsible
/// for closing it.
///
/// The mask is pre-validated by the caller against the per-session AIPC
/// allowlist via `policy::mask_is_allowed`; this function trusts the mask
/// and performs the FFI call.
///
/// The function is `safe` because the caller has already validated `handle`
/// is a live mutex kernel-object handle owned by this process (the supervisor
/// opened it via `CreateMutexW`); `DuplicateHandle` itself is the only FFI
/// the function performs and the unsafe contract for that call is documented
/// inside.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn broker_mutex_to_process(
    handle: HANDLE,
    target_process: BrokerTargetProcess,
    mask: u32,
) -> Result<ResourceGrant> {
    let mut duplicated: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live mutex kernel-object handle owned by the
    // supervisor. `target_process.raw()` was wrapped from a live process
    // handle. `duplicated` points to writable storage. `mask` was validated
    // by the caller against the per-session allowlist via
    // `policy::mask_is_allowed` before this call. `dwOptions = 0` (NOT
    // DUPLICATE_SAME_ACCESS) because we MAP DOWN — the supervisor source
    // may have full MUTEX_ALL_ACCESS but the child only gets the validated
    // subset (T-18-01-11 mitigation).
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            target_process.raw(),
            &mut duplicated,
            mask,
            0,
            0,
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (mutex, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ResourceGrant::duplicated_windows_mutex_handle(
        duplicated as usize as u64,
        mask,
    ))
}

/// Duplicate a Windows Job Object handle into the target process with the
/// caller-validated access `mask`.
///
/// Per CONTEXT.md D-05 footnote (Phase 18 AIPC-01): the supervisor MUST refuse
/// to broker its own `containment_job` HANDLE regardless of profile widening or
/// the access mask. That structural runtime guard lives in the dispatcher
/// (`handle_job_object_request`) which uses `CompareObjectHandles`; this broker
/// function trusts that guard has already fired BEFORE the call. The mask
/// itself is pre-validated by the caller against the per-session AIPC
/// allowlist via `policy::mask_is_allowed`.
///
/// Per CONTEXT.md D-10: caller closes the supervisor source AFTER
/// `send_response` returns; this function does NOT close the source. The
/// child owns the duplicated handle.
///
/// `dwOptions = 0` (NOT `DUPLICATE_SAME_ACCESS`) — must MAP DOWN. The supervisor
/// opens the source Job Object with `JOB_OBJECT_ALL_ACCESS`; the child only
/// receives the validated subset (T-18-03-01 mitigation).
///
/// The function is `safe` because the caller has already validated `handle`
/// is a live Job Object HANDLE owned by this process and that it is NOT the
/// supervisor's own containment Job (see `handle_job_object_request`);
/// `DuplicateHandle` itself is the only FFI the function performs and the
/// unsafe contract for that call is documented inside.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn broker_job_object_to_process(
    handle: HANDLE,
    target_process: BrokerTargetProcess,
    mask: u32,
) -> Result<ResourceGrant> {
    let mut duplicated: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live Job Object HANDLE owned by the supervisor
    // (opened via OpenJobObjectW in the dispatcher with JOB_OBJECT_ALL_ACCESS).
    // The dispatcher additionally fired the containment-Job runtime guard
    // (CompareObjectHandles against runtime.containment_job) BEFORE this call,
    // so this HANDLE is guaranteed NOT to be the supervisor's own containment
    // Job (T-18-03-01 mitigation). `target_process.raw()` was wrapped from a
    // live process handle. `duplicated` points to writable storage. `mask`
    // was pre-validated by the caller against the per-session allowlist via
    // `policy::mask_is_allowed`. `dwOptions = 0` (NOT DUPLICATE_SAME_ACCESS)
    // because we MAP DOWN — the supervisor source has JOB_OBJECT_ALL_ACCESS
    // but the child only gets the validated subset (T-18-03-01 mitigation).
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            target_process.raw(),
            &mut duplicated,
            mask,
            0,
            0, // dwOptions = 0 — MAP DOWN
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (Job Object, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ResourceGrant::duplicated_windows_job_object_handle(
        duplicated as usize as u64,
        mask,
    ))
}

/// Duplicate a Windows named-pipe handle into the target process with access
/// mapped DOWN from the supervisor source's `PIPE_ACCESS_DUPLEX` to the
/// requested `direction`. The mask is `GENERIC_READ`, `GENERIC_WRITE`, or
/// `GENERIC_READ | GENERIC_WRITE` per `direction`.
///
/// `dwOptions = 0` (NOT `DUPLICATE_SAME_ACCESS`) — must MAP DOWN. The supervisor
/// source has full PIPE_ACCESS_DUPLEX and `DUPLICATE_SAME_ACCESS` would
/// over-grant.
///
/// Per CONTEXT.md D-10 (Phase 18 AIPC-01): caller closes the supervisor source
/// AFTER `send_response` returns; this function does NOT close the source.
///
/// The function is `safe` because the caller has already validated `handle` is
/// a live named-pipe HANDLE owned by the supervisor (via [`bind_aipc_pipe`]);
/// `DuplicateHandle` itself is the only FFI the function performs and the
/// unsafe contract for that call is documented inside.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn broker_pipe_to_process(
    handle: HANDLE,
    target_process: BrokerTargetProcess,
    direction: PipeDirection,
) -> Result<ResourceGrant> {
    let mask = match direction {
        PipeDirection::Read => GENERIC_READ,
        PipeDirection::Write => GENERIC_WRITE,
        PipeDirection::ReadWrite => GENERIC_READ | GENERIC_WRITE,
    };
    let mut duplicated: HANDLE = std::ptr::null_mut();
    // SAFETY: `handle` is a live pipe-end HANDLE owned by the supervisor
    // (created via CreateNamedPipeW in `bind_aipc_pipe`).
    // `target_process.raw()` was wrapped from a live process handle.
    // `duplicated` points to writable storage. `mask` is derived from the
    // validated direction enum. `dwOptions = 0` is critical: the supervisor
    // source has full PIPE_ACCESS_DUPLEX and `DUPLICATE_SAME_ACCESS` would
    // over-grant — we MAP DOWN to the requested direction.
    let ok = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            target_process.raw(),
            &mut duplicated,
            mask,
            0,
            0, // dwOptions = 0 — MAP DOWN
        )
    };
    if ok == 0 || duplicated.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "DuplicateHandle (pipe, mask=0x{mask:08x}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ResourceGrant::duplicated_windows_pipe_handle(
        duplicated as usize as u64,
        direction,
    ))
}

/// Serialize a supervisor-owned `SOCKET` into a `WSAPROTOCOL_INFOW` blob the
/// target child process can consume via
/// `WSASocketW(FROM_PROTOCOL_INFO, ...)`.
///
/// The blob is single-use and bound to `target_pid` at duplication time per
/// the Microsoft API contract (RESEARCH Landmines § Socket).
///
/// Per CONTEXT.md D-10 + RESEARCH Landmines § Socket: caller closes the
/// supervisor's source `SOCKET` via `closesocket(s)` AFTER `send_response`
/// returns. If the supervisor exits before the child consumes the blob, the
/// underlying socket is leaked (the kernel keeps the socket alive until ALL
/// descriptors close, and the duplicated descriptor only materializes when
/// the child calls `WSASocketW(FROM_PROTOCOL_INFO, ...)`).
pub fn broker_socket_to_process(
    socket: SOCKET,
    _target_process: BrokerTargetProcess,
    target_pid: u32,
    role: SocketRole,
) -> Result<ResourceGrant> {
    if socket == INVALID_SOCKET {
        return Err(NonoError::SandboxInit(
            "WSADuplicateSocketW failed: source socket is INVALID_SOCKET".to_string(),
        ));
    }
    // SAFETY: zeroing a POD struct is well-defined; `WSAPROTOCOL_INFOW` is a
    // ~372-byte plain-old-data struct per the Microsoft API contract.
    let mut proto_info: WSAPROTOCOL_INFOW = unsafe { std::mem::zeroed() };
    // SAFETY: `socket` is a live SOCKET created by the supervisor with
    // WSASocketW (validated as not INVALID_SOCKET above). `target_pid` is the
    // validated child PID. `proto_info` points to writable 372-byte storage.
    // WSADuplicateSocketW serializes the socket capability into proto_info;
    // the result is single-use by the target PID.
    let rc = unsafe { WSADuplicateSocketW(socket, target_pid, &mut proto_info) };
    if rc != 0 {
        return Err(NonoError::SandboxInit(format!(
            "WSADuplicateSocketW failed (target_pid={target_pid}): {}",
            std::io::Error::last_os_error()
        )));
    }
    // Serialize the WSAPROTOCOL_INFOW struct to a Vec<u8> for wire transport.
    // SAFETY: `proto_info` is a live POD struct; we read its bytes as an
    // immutable byte slice of the struct's exact size (~372 bytes on x64).
    let bytes: Vec<u8> = unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!(proto_info) as *const u8,
            std::mem::size_of::<WSAPROTOCOL_INFOW>(),
        )
        .to_vec()
    };
    Ok(ResourceGrant::socket_protocol_info_blob(bytes, role))
}

/// Create a named pipe for AIPC broker handoff with Low Integrity SDDL
/// (mirrors Phase 11 `CAPABILITY_PIPE_SDDL`). Returns the supervisor-side
/// pipe `HANDLE` the dispatcher then duplicates into the child via
/// [`broker_pipe_to_process`].
///
/// The pipe is created as `PIPE_ACCESS_DUPLEX` server-side; the broker maps
/// the access DOWN to the requested direction at `DuplicateHandle` time, so
/// the supervisor source can serve any direction the child requests. The
/// `_direction` parameter is currently informational (kept in the signature
/// for forward-compat audit enrichment).
///
/// `canonical_name` MUST be the server-canonicalized
/// `\\.\pipe\nono-aipc-<user_session_id>-<sanitized_name>` shape per
/// CONTEXT.md `<specifics>` line 168 — the caller is responsible for that
/// canonicalization before reaching this helper.
pub fn bind_aipc_pipe(canonical_name: &str, _direction: PipeDirection) -> Result<HANDLE> {
    // AIPC pipes do not face WRITE_RESTRICTED + restricting-SID children; the
    // target process is the child's own spawned AIPC peer which inherits the
    // normal user token. The byte-identical pre-fix SDDL is correct here.
    let (sa, _sd_guard) = build_low_integrity_security_attributes(None)?;
    let wide_name = to_wide(canonical_name);

    // SAFETY: `wide_name` is a valid null-terminated UTF-16 string with a
    // lifetime that outlives the FFI call. `sa` carries a security descriptor
    // owned by `_sd_guard` for the duration of this call. PIPE_ACCESS_DUPLEX
    // with PIPE_UNLIMITED_INSTANCES is the standard server-side AIPC pipe
    // shape so multiple AIPC capability requests can target distinct
    // canonical names without competing for the single-instance slot the
    // supervisor control pipe uses. Returned HANDLE is owned by the caller
    // and must be closed via CloseHandle (caller's responsibility per D-10).
    let handle: HANDLE = unsafe {
        CreateNamedPipeW(
            wide_name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            &sa,
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "CreateNamedPipeW(\"{canonical_name}\") failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(handle)
}

impl Drop for SupervisorSocket {
    fn drop(&mut self) {
        if self.disconnect_on_drop {
            let raw = self.writer.as_raw_handle();
            if !raw.is_null() {
                // SAFETY: The handle comes from this `File` and remains valid for
                // the duration of `drop`. Disconnecting the server end is the
                // Windows-equivalent cleanup for a bound named pipe instance.
                unsafe {
                    let _ = DisconnectNamedPipe(raw as HANDLE);
                }
            }
        }
        if let Some(path) = self.cleanup_rendezvous_path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn unique_pair_name() -> Result<String> {
    let mut nonce = [0u8; 16];
    random_fill(&mut nonce)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to generate pipe nonce: {e}")))?;
    let suffix = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "windows-supervisor-anon-{}-{suffix}",
        std::process::id()
    ))
}

fn explicit_pipe_name(path: &Path) -> Option<String> {
    let display = path.to_string_lossy();
    if display.starts_with(r"\\.\pipe\") {
        Some(display.into_owned())
    } else {
        None
    }
}

fn create_nonce_hex() -> Result<String> {
    let mut nonce = [0u8; 16];
    random_fill(&mut nonce)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to generate pipe nonce: {e}")))?;
    Ok(nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

fn pipe_name_from_rendezvous_path(path: &Path, nonce_hex: &str) -> String {
    let mut hasher = Sha256::new();
    let display = path.to_string_lossy();
    hasher.update(display.as_bytes());
    let digest = hasher.finalize();
    let short_hash = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("supervisor");
    let safe_leaf = leaf
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();

    format!(r"\\.\pipe\nono-{}-{short_hash}-{nonce_hex}", safe_leaf)
}

fn prepare_bind_pipe_name(path: &Path) -> Result<(String, Option<PathBuf>)> {
    if let Some(pipe_name) = explicit_pipe_name(path) {
        return Ok((pipe_name, None));
    }

    let nonce_hex = create_nonce_hex()?;
    Ok((
        pipe_name_from_rendezvous_path(path, &nonce_hex),
        Some(path.to_path_buf()),
    ))
}

fn resolve_connect_pipe_name(path: &Path) -> Result<PipeRendezvousInfo> {
    if let Some(pipe_name) = explicit_pipe_name(path) {
        return Ok(PipeRendezvousInfo {
            pipe_name,
            server_pid: 0,
        });
    }

    read_pipe_rendezvous(path)
}

fn write_pipe_rendezvous(path: &Path, pipe_name: &str) -> Result<()> {
    if explicit_pipe_name(path).is_some() {
        return Ok(());
    }

    let parent = path.parent().ok_or_else(|| {
        NonoError::SandboxInit(format!(
            "Windows supervisor rendezvous path {} has no parent directory",
            path.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to create Windows supervisor rendezvous directory {}: {e}",
            parent.display()
        ))
    })?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            NonoError::SandboxInit(format!(
                "Failed to publish Windows supervisor rendezvous {}: {e}",
                path.display()
            ))
        })?;
    let payload = format!("{pipe_name}\n{}", std::process::id());
    file.write_all(payload.as_bytes()).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to write Windows supervisor rendezvous {}: {e}",
            path.display()
        ))
    })?;
    file.flush().map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to flush Windows supervisor rendezvous {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}

fn read_pipe_rendezvous(path: &Path) -> Result<PipeRendezvousInfo> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to read Windows supervisor pipe rendezvous {}: {e}. \
Ensure the supervisor created the control channel before launching the child.",
            path.display()
        ))
    })?;
    let mut lines = contents.lines();
    let pipe_name = lines.next().unwrap_or_default().trim();
    if !pipe_name.starts_with(r"\\.\pipe\") {
        return Err(NonoError::SandboxInit(format!(
            "Windows supervisor pipe rendezvous {} did not contain a valid pipe name",
            path.display()
        )));
    }
    let server_pid = lines
        .next()
        .ok_or_else(|| {
            NonoError::SandboxInit(format!(
                "Windows supervisor pipe rendezvous {} did not include a server PID",
                path.display()
            ))
        })?
        .trim()
        .parse::<u32>()
        .map_err(|e| {
            NonoError::SandboxInit(format!(
                "Windows supervisor pipe rendezvous {} contained an invalid server PID: {e}",
                path.display()
            ))
        })?;
    Ok(PipeRendezvousInfo {
        pipe_name: pipe_name.to_string(),
        server_pid,
    })
}

/// Scope guard that frees a PSECURITY_DESCRIPTOR on drop via `LocalFree`.
struct SecurityDescriptorGuard(PSECURITY_DESCRIPTOR);

impl Drop for SecurityDescriptorGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                // SAFETY: `self.0` was obtained from
                // `ConvertStringSecurityDescriptorToSecurityDescriptorW`,
                // which documents `LocalFree` as the correct release routine.
                LocalFree(self.0 as _);
            }
        }
    }
}

/// Validate a per-session restricting SID string before embedding it in SDDL.
///
/// The SID is concatenated directly into an SDDL template, so any unexpected
/// character (quote, parenthesis, semicolon, whitespace, NUL byte) could let a
/// malicious or corrupted input subvert the resulting DACL. This function
/// enforces a conservative allow-list matching the shape produced by
/// `nono-cli`'s `generate_session_sid`:
///
/// - Non-empty, no longer than [`SESSION_SID_MAX_LEN`] characters.
/// - Begins with ASCII `S-1-` (so `ConvertStringSidToSidW` will accept it and
///   nothing else can prefix an alternate SDDL token).
/// - Every byte after the `S-` prefix is an ASCII digit or `-`.
///
/// Returns an error on any violation — the caller MUST treat it as fatal and
/// MUST NOT silently fall back to the no-SID SDDL path. Fail-closed is
/// mandatory per CLAUDE.md § "Fail Secure".
fn validate_session_sid_for_sddl(sid: &str) -> Result<()> {
    if sid.is_empty() {
        return Err(NonoError::SandboxInit(
            "malformed session SID: empty string".to_string(),
        ));
    }
    if sid.len() > SESSION_SID_MAX_LEN {
        return Err(NonoError::SandboxInit(format!(
            "malformed session SID: length {} exceeds maximum {}",
            sid.len(),
            SESSION_SID_MAX_LEN
        )));
    }
    if !sid.starts_with("S-1-") {
        return Err(NonoError::SandboxInit(format!(
            "malformed session SID: must start with \"S-1-\" (got {sid:?})"
        )));
    }
    // Character-class check on the tail (post `S-`): digits and hyphens only.
    // `.bytes()` avoids any Unicode surprises — every accepted byte is ASCII.
    let tail = &sid[2..];
    for b in tail.bytes() {
        if !(b.is_ascii_digit() || b == b'-') {
            return Err(NonoError::SandboxInit(format!(
                "malformed session SID: contains non-digit non-hyphen character (got {sid:?})"
            )));
        }
    }
    Ok(())
}

/// Construct the SDDL string for the capability pipe, optionally appending an
/// ACE that grants the per-session restricting SID `FILE_GENERIC_READ |
/// FILE_GENERIC_WRITE | SYNCHRONIZE`.
///
/// With `session_sid = None`, returns exactly `CAPABILITY_PIPE_SDDL` — this
/// preserves byte-identical behavior for in-process tests and AIPC pipe
/// callers that never face a WRITE_RESTRICTED child.
fn build_capability_pipe_sddl(session_sid: Option<&str>) -> Result<String> {
    match session_sid {
        None => Ok(CAPABILITY_PIPE_SDDL.to_string()),
        Some(sid) => {
            validate_session_sid_for_sddl(sid)?;
            // Insert the restricting-SID ACE BEFORE the SACL. SDDL requires
            // the DACL (`D:`) section to precede the SACL (`S:`) section.
            Ok(format!(
                "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;{mask};;;{sid})S:(ML;;NW;;;LW)",
                mask = CAPABILITY_PIPE_RESTRICTING_SID_MASK,
            ))
        }
    }
}

fn build_low_integrity_security_attributes(
    session_sid: Option<&str>,
) -> Result<(SECURITY_ATTRIBUTES, SecurityDescriptorGuard)> {
    let sddl_str = build_capability_pipe_sddl(session_sid)?;
    let sddl_u16: Vec<u16> = sddl_str.encode_utf16().chain(std::iter::once(0)).collect();
    let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: `sddl_u16` is a valid null-terminated UTF-16 string and
        // `security_descriptor` points to writable storage. The returned
        // descriptor must be freed via `LocalFree`, handled by the guard.
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_u16.as_ptr(),
            SDDL_REVISION_1,
            &mut security_descriptor,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || security_descriptor.is_null() {
        return Err(NonoError::SandboxInit(format!(
            "Failed to convert capability pipe SDDL to security descriptor: {}",
            std::io::Error::last_os_error()
        )));
    }
    let guard = SecurityDescriptorGuard(security_descriptor);
    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security_descriptor as _,
        bInheritHandle: 0,
    };
    Ok((sa, guard))
}

fn create_low_integrity_named_pipe(pipe_name: &str, session_sid: Option<&str>) -> Result<HANDLE> {
    let (sa, _sd_guard) = build_low_integrity_security_attributes(session_sid)?;
    let wide_name = to_wide(pipe_name);

    // SAFETY: `wide_name` is a valid null-terminated UTF-16 string. `sa` carries
    // a security descriptor freed by `_sd_guard` on function return. The
    // returned handle is owned by the caller.
    let handle = unsafe {
        CreateNamedPipeW(
            wide_name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            MAX_MESSAGE_SIZE,
            MAX_MESSAGE_SIZE,
            PIPE_CONNECT_TIMEOUT_MS,
            &sa,
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(NonoError::SandboxInit(format!(
            "Failed to create low-integrity Windows supervisor pipe {pipe_name}: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok(handle)
}

fn create_named_pipe(pipe_name: &str, first_instance: bool) -> Result<HANDLE> {
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first_instance {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let wide_name = to_wide(pipe_name);

    // SAFETY: `wide_name` is a valid null-terminated UTF-16 string. We request a
    // single duplex byte-mode instance with no external pointers.
    let handle = unsafe {
        CreateNamedPipeW(
            wide_name.as_ptr(),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            MAX_MESSAGE_SIZE,
            MAX_MESSAGE_SIZE,
            PIPE_CONNECT_TIMEOUT_MS,
            std::ptr::null(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(NonoError::SandboxInit(format!(
            "Failed to create Windows supervisor pipe {pipe_name}: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok(handle)
}

fn finalize_server_connection(server_handle: HANDLE, pipe_name: &str) -> Result<File> {
    // SAFETY: `server_handle` was returned by `CreateNamedPipeW` and is still
    // owned by this function until converted into `OwnedHandle`.
    let connected = unsafe { ConnectNamedPipe(server_handle, std::ptr::null_mut()) };
    if connected == 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            // SAFETY: `server_handle` is a live handle that must be reclaimed on error.
            drop(unsafe { OwnedHandle::from_raw_handle(server_handle) });
            return Err(NonoError::SandboxInit(format!(
                "Failed to accept Windows supervisor pipe connection on {pipe_name}: {err}. \
Ensure the child process received the correct pipe name and startup token."
            )));
        }
    }

    Ok(file_from_handle(server_handle))
}

fn connect_named_pipe(rendezvous: &PipeRendezvousInfo) -> Result<File> {
    let wide_name = to_wide(&rendezvous.pipe_name);
    let mut last_error: Option<std::io::Error> = None;

    for _ in 0..3 {
        // SAFETY: `wide_name` is a valid null-terminated UTF-16 string. We open
        // the pipe for duplex access and receive an owned OS handle on success.
        let handle = unsafe {
            CreateFileW(
                wide_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle != INVALID_HANDLE_VALUE {
            let file = file_from_handle(handle);
            if rendezvous.server_pid != 0 {
                verify_connected_server_pid(file.as_raw_handle() as HANDLE, rendezvous)?;
            }
            return Ok(file);
        }

        let err = std::io::Error::last_os_error();
        if matches!(
            err.raw_os_error(),
            Some(code) if code == ERROR_PIPE_BUSY as i32 || code == ERROR_FILE_NOT_FOUND as i32
        ) {
            // SAFETY: `wide_name` remains valid for the duration of the call.
            let waited = unsafe { WaitNamedPipeW(wide_name.as_ptr(), PIPE_CONNECT_TIMEOUT_MS) };
            if waited != 0 {
                continue;
            }
            last_error = Some(std::io::Error::last_os_error());
            continue;
        }

        return Err(NonoError::SandboxInit(format!(
            "Failed to connect to Windows supervisor pipe {}: {err}. \
Ensure the supervisor created the control channel before launching the child.",
            rendezvous.pipe_name
        )));
    }

    let err = last_error.unwrap_or_else(std::io::Error::last_os_error);
    Err(NonoError::SandboxInit(format!(
        "Timed out waiting for Windows supervisor pipe {}: {err}. \
Ensure the parent process is listening before the child attempts to connect.",
        rendezvous.pipe_name
    )))
}

fn verify_connected_server_pid(handle: HANDLE, rendezvous: &PipeRendezvousInfo) -> Result<()> {
    let mut actual_server_pid = 0u32;
    let ok = unsafe {
        // SAFETY: `handle` is a live pipe handle returned by `CreateFileW` and
        // `actual_server_pid` points to writable storage for the queried PID.
        GetNamedPipeServerProcessId(handle, &mut actual_server_pid)
    };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to verify Windows supervisor pipe server PID for {}: {}",
            rendezvous.pipe_name,
            std::io::Error::last_os_error()
        )));
    }
    if actual_server_pid != rendezvous.server_pid {
        return Err(NonoError::SandboxInit(format!(
            "Windows supervisor pipe peer validation failed for {}: expected server PID {}, got {}",
            rendezvous.pipe_name, rendezvous.server_pid, actual_server_pid
        )));
    }
    Ok(())
}

fn file_from_handle(handle: HANDLE) -> File {
    // SAFETY: `handle` is a valid owned Windows handle returned by a Win32 API
    // in this module. Converting it into `OwnedHandle` transfers ownership to Rust.
    let owned = unsafe { OwnedHandle::from_raw_handle(handle) };
    File::from(owned)
}

fn create_anonymous_pipe() -> Result<(File, File)> {
    let mut read_handle: HANDLE = std::ptr::null_mut();
    let mut write_handle: HANDLE = std::ptr::null_mut();

    // SAFETY: We pass valid out-pointers for the read and write handles and do
    // not request inherited handles yet. On success, both handles are owned by
    // this function and immediately wrapped in `File`.
    let created = unsafe { CreatePipe(&mut read_handle, &mut write_handle, std::ptr::null(), 0) };
    if created == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to create Windows anonymous pipe pair: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok((
        file_from_handle(read_handle),
        file_from_handle(write_handle),
    ))
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::capability::AccessMode;
    use crate::supervisor::types::{
        ApprovalDecision, CapabilityRequest, ResourceTransferKind, SupervisorMessage,
        SupervisorResponse,
    };
    use std::path::PathBuf;
    #[test]
    fn test_pipe_pair_roundtrip() {
        let (mut supervisor, mut child) =
            SupervisorSocket::pair().expect("Failed to create named pipe pair");

        let request = CapabilityRequest {
            request_id: "req-001".to_string(),
            path: r"C:\tmp\test.txt".into(),
            access: AccessMode::Read,
            reason: Some("test access".to_string()),
            child_pid: 12345,
            session_id: "sess-001".to_string(),
            session_token: "test-token".to_string(),
            kind: crate::supervisor::types::HandleKind::File,
            target: None,
            access_mask: 0,
        };

        child
            .send_message(&SupervisorMessage::Request(request.clone()))
            .expect("Failed to send request");

        let msg = supervisor
            .recv_message()
            .expect("Failed to receive request");
        match msg {
            SupervisorMessage::Request(req) => {
                assert_eq!(req.request_id, "req-001");
                assert_eq!(req.path, PathBuf::from(r"C:\tmp\test.txt"));
                assert_eq!(req.child_pid, 12345);
            }
            other => panic!("Expected Request, got {other:?}"),
        }

        supervisor
            .send_response(&SupervisorResponse::Decision {
                request_id: "req-001".to_string(),
                decision: ApprovalDecision::Granted,
                grant: Some(ResourceGrant::duplicated_windows_file_handle(
                    0x1234,
                    AccessMode::Read,
                )),
            })
            .expect("Failed to send response");

        let resp = child.recv_response().expect("Failed to receive response");
        match resp {
            SupervisorResponse::Decision {
                request_id,
                decision,
                grant,
            } => {
                assert_eq!(request_id, "req-001");
                assert!(decision.is_granted());
                let grant = grant.expect("grant metadata should be present");
                assert_eq!(
                    grant.transfer,
                    ResourceTransferKind::DuplicatedWindowsHandle
                );
                assert_eq!(grant.raw_handle, Some(0x1234));
            }
            other => panic!("Expected Decision, got {other:?}"),
        }
    }

    #[test]
    fn test_message_too_large() {
        let (mut supervisor, _child) =
            SupervisorSocket::pair().expect("Failed to create named pipe pair");

        let large_payload = vec![0u8; (MAX_MESSAGE_SIZE as usize) + 1];
        let result = supervisor.write_frame(&large_payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_missing_pipe_returns_actionable_diagnostic() {
        let path = PathBuf::from(format!(
            r"C:\tmp\nono-win-missing-{}-{}",
            std::process::id(),
            1
        ));
        let err = SupervisorSocket::connect(&path).expect_err("Connect should fail");
        let message = err.to_string();
        assert!(message.contains("Windows supervisor pipe"));
        assert!(
            message.contains("Ensure the supervisor created the control channel")
                || message.contains("Ensure the parent process is listening")
                || message.contains("Timed out waiting for Windows supervisor pipe")
        );
    }

    #[test]
    fn test_rendezvous_paths_publish_nonce_backed_pipe_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rendezvous = dir.path().join("supervisor.pipe");

        let (pipe_name_one, cleanup_one) =
            prepare_bind_pipe_name(&rendezvous).expect("prepare first pipe");
        let (pipe_name_two, cleanup_two) =
            prepare_bind_pipe_name(&rendezvous).expect("prepare second pipe");

        assert_ne!(pipe_name_one, pipe_name_two);
        assert_eq!(cleanup_one.as_deref(), Some(rendezvous.as_path()));
        assert_eq!(cleanup_two.as_deref(), Some(rendezvous.as_path()));
        assert!(pipe_name_one.starts_with(r"\\.\pipe\nono-"));
        assert!(pipe_name_two.starts_with(r"\\.\pipe\nono-"));
    }

    #[test]
    fn test_explicit_pipe_paths_are_preserved() {
        let explicit = PathBuf::from(r"\\.\pipe\nono-explicit");
        let (pipe_name, cleanup_path) =
            prepare_bind_pipe_name(&explicit).expect("prepare explicit pipe");

        assert_eq!(pipe_name, r"\\.\pipe\nono-explicit");
        assert!(cleanup_path.is_none());
    }

    #[test]
    fn test_rendezvous_roundtrip_uses_published_pipe_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rendezvous = dir.path().join("socket.info");
        let pipe_name = r"\\.\pipe\nono-test-roundtrip";

        write_pipe_rendezvous(&rendezvous, pipe_name).expect("write rendezvous");
        let resolved = resolve_connect_pipe_name(&rendezvous).expect("resolve rendezvous");

        assert_eq!(resolved.pipe_name, pipe_name);
        assert_eq!(resolved.server_pid, std::process::id());
    }

    #[test]
    fn test_explicit_pipe_path_skips_server_pid_expectation() {
        let explicit = PathBuf::from(r"\\.\pipe\nono-explicit-connect");
        let resolved = resolve_connect_pipe_name(&explicit).expect("resolve explicit");

        assert_eq!(resolved.pipe_name, r"\\.\pipe\nono-explicit-connect");
        assert_eq!(resolved.server_pid, 0);
    }

    #[test]
    fn test_read_pipe_rendezvous_rejects_missing_server_pid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rendezvous = dir.path().join("socket.info");
        std::fs::write(&rendezvous, r"\\.\pipe\nono-test-only").expect("write rendezvous");

        let err = read_pipe_rendezvous(&rendezvous).expect_err("missing pid should fail");
        assert!(err.to_string().contains("did not include a server PID"));
    }

    #[test]
    fn test_bind_low_integrity_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rendezvous = dir.path().join("lowint.pipe");
        let rendezvous_server = rendezvous.clone();
        let rendezvous_client = rendezvous.clone();

        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();

        let server_thread = std::thread::spawn(move || {
            let mut server = SupervisorSocket::bind_low_integrity(&rendezvous_server)
                .expect("bind low integrity");
            let msg = server.recv_message().expect("recv message");
            match msg {
                SupervisorMessage::Request(req) => {
                    assert_eq!(req.request_id, "lowint-req");
                    assert_eq!(req.session_token, "tok");
                }
                other => panic!("unexpected message: {other:?}"),
            }
            server
                .send_response(&SupervisorResponse::Decision {
                    request_id: "lowint-req".to_string(),
                    decision: ApprovalDecision::Granted,
                    grant: None,
                })
                .expect("send response");
            // Wait for the client to confirm it has read the response before
            // dropping the pipe, which would otherwise tear down the client's
            // read side prematurely.
            let _ = done_rx.recv();
            drop(server);
        });

        // Give the server a moment to publish the rendezvous file.
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut client = SupervisorSocket::connect(&rendezvous_client).expect("connect");
        let request = CapabilityRequest {
            request_id: "lowint-req".to_string(),
            path: r"C:\tmp\lowint.txt".into(),
            access: AccessMode::Read,
            reason: None,
            child_pid: 12345,
            session_id: "sess-lowint".to_string(),
            session_token: "tok".to_string(),
            kind: crate::supervisor::types::HandleKind::File,
            target: None,
            access_mask: 0,
        };
        client
            .send_message(&SupervisorMessage::Request(request))
            .expect("send request");
        match client.recv_response().expect("recv response") {
            SupervisorResponse::Decision { decision, .. } => {
                assert!(decision.is_granted());
            }
            other => panic!("unexpected response: {other:?}"),
        }

        done_tx.send(()).expect("signal server");
        server_thread.join().expect("server thread");
    }

    // ---------------------------------------------------------------------
    // Regression test: capability pipe must admit a WRITE_RESTRICTED +
    // per-session-restricting-SID child (Phase 13 token shape). Verifies the
    // fix for the `supervisor-pipe-access-denied` debug session — without the
    // dynamic `(A;;0x120089;;;<session_sid>)` DACL ACE, the second-pass access
    // check against the restricting SID set fails with ERROR_ACCESS_DENIED.
    // ---------------------------------------------------------------------

    /// Pseudo-unique session SID for the regression test. Emulates the shape
    /// produced by `nono-cli::exec_strategy_windows::restricted_token::generate_session_sid`
    /// (`S-1-5-117-<u32>-<u32>-<u32>-<u32>`) without pulling `uuid` into the
    /// library `dev-dependencies`.
    #[cfg(target_os = "windows")]
    fn fabricate_session_sid(seed: u32) -> String {
        let pid = std::process::id();
        // SAFETY-adjacent: values are bounded u32s; the SID stays within
        // SESSION_SID_MAX_LEN and contains only ASCII digits + hyphens.
        // 4 subauthorities after the `S-1-5-117-` prefix — matches the shape
        // produced by `nono-cli::generate_session_sid` exactly.
        format!(
            "S-1-5-117-{pid}-{seed}-{}-{}",
            seed.wrapping_mul(2654435761),
            seed.wrapping_add(0x9E3779B1),
        )
    }

    /// Build a WRITE_RESTRICTED token carrying `session_sid` as its single
    /// restricting SID. Returns a raw HANDLE the caller closes via
    /// `CloseHandle`. Mirrors
    /// `nono-cli::exec_strategy_windows::restricted_token::create_restricted_token_with_sid`
    /// but lives in-crate so this regression test doesn't depend on nono-cli.
    #[cfg(target_os = "windows")]
    fn build_write_restricted_token(session_sid: &str) -> HANDLE {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
        use windows_sys::Win32::Security::{
            CreateRestrictedToken, SID_AND_ATTRIBUTES, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
            TOKEN_IMPERSONATE, TOKEN_QUERY, WRITE_RESTRICTED,
        };
        use windows_sys::Win32::System::Threading::OpenProcessToken;

        let mut h_current: HANDLE = std::ptr::null_mut();
        // SAFETY: OpenProcessToken writes a valid handle or returns 0.
        let ok = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_IMPERSONATE,
                &mut h_current,
            )
        };
        assert_ne!(ok, 0, "OpenProcessToken failed: {}", unsafe {
            GetLastError()
        });

        let sid_u16: Vec<u16> = session_sid
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut sid_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: sid_u16 is valid null-terminated UTF-16; sid_ptr is a valid
        // writable out-pointer. On success sid_ptr must be freed via LocalFree.
        let ok = unsafe { ConvertStringSidToSidW(sid_u16.as_ptr(), &mut sid_ptr) };
        assert_ne!(
            ok,
            0,
            "ConvertStringSidToSidW failed for {session_sid:?}: {}",
            unsafe { GetLastError() }
        );

        let sid_restrict = SID_AND_ATTRIBUTES {
            Sid: sid_ptr,
            Attributes: 0,
        };
        let mut h_restricted: HANDLE = std::ptr::null_mut();
        // SAFETY: h_current is a live token handle; sid_restrict references
        // sid_ptr (live until LocalFree below); h_restricted is a writable
        // out-pointer. WRITE_RESTRICTED confines the second-pass DACL check to
        // WRITE accesses only (Phase 13 contract).
        let ok = unsafe {
            CreateRestrictedToken(
                h_current,
                WRITE_RESTRICTED,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &sid_restrict,
                &mut h_restricted,
            )
        };

        // SAFETY: sid_ptr was returned by ConvertStringSidToSidW above and is
        // not retained after this point (CreateRestrictedToken has copied it
        // into the new token).
        unsafe { LocalFree(sid_ptr as _) };
        // SAFETY: h_current is a live token handle we opened above.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(h_current) };

        assert_ne!(ok, 0, "CreateRestrictedToken failed: {}", unsafe {
            GetLastError()
        });
        assert!(!h_restricted.is_null(), "restricted token handle is NULL");
        h_restricted
    }

    /// Parse the SDDL produced by [`build_capability_pipe_sddl`] into a
    /// security descriptor and walk the DACL, returning the list of
    /// `(sid_string, access_mask)` tuples. This lets the regression test
    /// verify the *structural shape* of the DACL — specifically that the
    /// per-session restricting-SID ACE is present with the expected
    /// `FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE` (`0x12019F`)
    /// mask — without having to simulate a WRITE_RESTRICTED child process
    /// end-to-end (which requires `CreateProcessAsUserW` and is too heavy for
    /// a unit test). The production repro in
    /// `.planning/debug/supervisor-pipe-access-denied.md` is the teeth-level
    /// verification; this test guards against regressions in the SDDL builder.
    #[cfg(target_os = "windows")]
    fn extract_dacl_aces_from_sddl(sddl: &str) -> Vec<(String, u32)> {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::Security::Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        };
        use windows_sys::Win32::Security::{
            AclSizeInformation, GetAclInformation, GetSecurityDescriptorDacl, ACCESS_ALLOWED_ACE,
            ACL, ACL_SIZE_INFORMATION,
        };

        // Convert SDDL → security descriptor.
        let sddl_u16: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: sddl_u16 is valid null-terminated UTF-16; sd is writable.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl_u16.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(
            ok,
            0,
            "ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {}",
            unsafe { GetLastError() }
        );
        let _sd_guard = SecurityDescriptorGuard(sd);

        // Pull out the DACL.
        let mut present: i32 = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut defaulted: i32 = 0;
        // SAFETY: all out-pointers are valid writable stack storage. `sd` is
        // live for the lifetime of _sd_guard.
        let ok = unsafe { GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut defaulted) };
        assert_ne!(ok, 0, "GetSecurityDescriptorDacl failed");
        assert_ne!(present, 0, "expected DACL to be present");
        assert!(!dacl.is_null(), "expected non-NULL DACL pointer");

        // Walk the ACE list.
        let mut info: ACL_SIZE_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: info is writable; dacl is a live ACL pointer.
        let ok = unsafe {
            GetAclInformation(
                dacl,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        };
        assert_ne!(ok, 0, "GetAclInformation failed");

        let mut out = Vec::new();
        for i in 0..info.AceCount {
            let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            // SAFETY: dacl is live; ace_ptr is writable.
            let ok = unsafe { windows_sys::Win32::Security::GetAce(dacl, i, &mut ace_ptr) };
            assert_ne!(ok, 0, "GetAce({i}) failed");
            // ACCESS_ALLOWED_ACE is the prefix of the ACE struct for type 0.
            // SAFETY: GetAce returned a valid pointer into the ACL buffer.
            let ace = unsafe { &*(ace_ptr as *const ACCESS_ALLOWED_ACE) };
            // The SID immediately follows the ACCESS_ALLOWED_ACE's header +
            // AceSize fields. In windows-sys 0.59 ACCESS_ALLOWED_ACE is laid
            // out as `{ Header, Mask, SidStart: u32 }` where `SidStart` is the
            // first DWORD of the variable-length SID.
            let sid_ptr = std::ptr::addr_of!(ace.SidStart) as *mut std::ffi::c_void;
            let mut sid_str_ptr: *mut u16 = std::ptr::null_mut();
            // SAFETY: sid_ptr points into the ACE's SID field; sid_str_ptr is
            // a writable out-pointer. Output must be freed via LocalFree.
            let ok = unsafe { ConvertSidToStringSidW(sid_ptr, &mut sid_str_ptr) };
            assert_ne!(ok, 0, "ConvertSidToStringSidW failed");
            // Read the null-terminated UTF-16 string.
            let mut len = 0usize;
            while unsafe { *sid_str_ptr.add(len) } != 0 {
                len += 1;
            }
            let slice = unsafe { std::slice::from_raw_parts(sid_str_ptr, len) };
            let sid_str = String::from_utf16_lossy(slice);
            unsafe { LocalFree(sid_str_ptr as _) };
            out.push((sid_str, ace.Mask));
        }
        out
    }

    /// **Regression test for `supervisor-pipe-access-denied`** (Phase 21 UAT
    /// G-01).
    ///
    /// Verifies that the capability pipe's DACL — built via
    /// [`build_capability_pipe_sddl`] with a per-session restricting SID —
    /// contains an explicit ACE granting `FILE_GENERIC_READ |
    /// FILE_GENERIC_WRITE | SYNCHRONIZE` (`0x12019F`) to that per-session
    /// SID. This is exactly the mask Windows requires for a
    /// WRITE_RESTRICTED child's `CreateFileW(pipe, GENERIC_READ |
    /// GENERIC_WRITE)` to pass the second-pass restricting-SID access check.
    ///
    /// The test walks the parsed DACL via `GetSecurityDescriptorDacl +
    /// GetAclInformation + GetAce + ConvertSidToStringSidW`, which guarantees
    /// the ACE was not silently dropped at SD-conversion time (our primary
    /// concern is an SDDL-injection or SID-format mismatch that would cause
    /// `ConvertStringSecurityDescriptorToSecurityDescriptorW` to skip the
    /// ACE).
    ///
    /// End-to-end verification of the WRITE_RESTRICTED child behavior
    /// requires `CreateProcessAsUserW` which is out of scope for a library
    /// unit test — that verification lives in the production repro
    /// documented at
    /// `.planning/debug/supervisor-pipe-access-denied.md`.
    #[test]
    #[cfg(target_os = "windows")]
    fn capability_pipe_admits_restricted_token_child_with_session_sid() {
        let session_sid = fabricate_session_sid(0xBADC0FFE);

        // Validate that a WRITE_RESTRICTED token carrying this SID as its
        // restricting SID can even be constructed. If the SID format (shape
        // produced by `nono-cli::generate_session_sid`) were rejected by
        // `CreateRestrictedToken`, the whole fix would be moot.
        let restricted_token = build_write_restricted_token(&session_sid);
        // SAFETY: restricted_token is owned only by this scope.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(restricted_token) };

        let sddl = build_capability_pipe_sddl(Some(&session_sid))
            .expect("build_capability_pipe_sddl for valid session SID must succeed");
        assert!(
            sddl.contains(&session_sid),
            "SDDL should embed the session SID verbatim: {sddl}"
        );

        // Walk the converted DACL and confirm the session-SID ACE is
        // present with the expected mask. Access mask 0x12019F ==
        // FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE — the
        // generic-mapped result of `GRGW` applied against the file generic
        // mapping at SD-conversion time.
        let aces = extract_dacl_aces_from_sddl(&sddl);
        let matching = aces
            .iter()
            .find(|(sid, _)| sid.eq_ignore_ascii_case(&session_sid))
            .unwrap_or_else(|| {
                panic!(
                    "session-SID ACE missing from DACL — SDDL-injection guard may be dropping \
                     the SID, or the generic-mnemonic mask was silently ignored. SDDL={sddl}, \
                     aces={aces:?}"
                )
            });
        assert_eq!(
            matching.1, 0x0012_019F,
            "session-SID ACE mask is 0x{:08X}, expected 0x0012019F (FILE_GENERIC_READ | \
             FILE_GENERIC_WRITE | SYNCHRONIZE). A different mask would break the \
             WRITE_RESTRICTED child's CreateFileW access check. SDDL={sddl}",
            matching.1
        );

        // Pre-fix guard: when session_sid is None, the DACL MUST NOT contain
        // an ACE for the restricting SID — the whole point of the fix is
        // that the pre-fix DACL lacked this ACE and denied WRITE_RESTRICTED
        // children. If this assertion fails, the `None` path regressed.
        let baseline_sddl = build_capability_pipe_sddl(None).expect("no-SID path must succeed");
        assert_eq!(baseline_sddl, CAPABILITY_PIPE_SDDL);
        let baseline_aces = extract_dacl_aces_from_sddl(&baseline_sddl);
        assert!(
            !baseline_aces
                .iter()
                .any(|(sid, _)| sid.starts_with("S-1-5-117-")),
            "baseline (no-SID) DACL must not contain any S-1-5-117-* ACE; aces={baseline_aces:?}"
        );
    }

    /// `validate_session_sid_for_sddl` must reject any input that could
    /// smuggle an extra SDDL token through string concatenation. This is the
    /// defense-in-depth unit test for the SDDL-injection guard.
    #[test]
    fn validate_session_sid_for_sddl_rejects_injection() {
        // Accepted shapes:
        assert!(validate_session_sid_for_sddl("S-1-5-117-1-2-3-4").is_ok());
        assert!(validate_session_sid_for_sddl("S-1-5-117-4294967295-0-0-0").is_ok());

        // Rejected shapes:
        assert!(validate_session_sid_for_sddl("").is_err());
        assert!(validate_session_sid_for_sddl("S-1-5-117").is_ok()); // digits + hyphens only — fine
        assert!(
            validate_session_sid_for_sddl("X-1-5-117-1").is_err(),
            "must start with S-1-"
        );
        assert!(
            validate_session_sid_for_sddl("S-1-5-117-1)(A;;GA;;;WD").is_err(),
            "injection via ACE"
        );
        assert!(
            validate_session_sid_for_sddl("S-1-5-117-1;A").is_err(),
            "semicolon"
        );
        assert!(
            validate_session_sid_for_sddl("S-1-5-117-1 ").is_err(),
            "trailing space"
        );
        assert!(
            validate_session_sid_for_sddl("S-1-5-117-\0").is_err(),
            "embedded NUL"
        );
        assert!(
            validate_session_sid_for_sddl("s-1-5-117-1").is_err(),
            "lowercase prefix"
        );
        let too_long = format!("S-1-5-117-{}", "1".repeat(SESSION_SID_MAX_LEN));
        assert!(
            validate_session_sid_for_sddl(&too_long).is_err(),
            "length cap"
        );
    }

    /// `build_capability_pipe_sddl(None)` must return the byte-identical
    /// pre-fix constant so in-process tests and AIPC pipe callers are not
    /// perturbed by the restricted-SID plumbing.
    #[test]
    fn build_capability_pipe_sddl_none_matches_constant() {
        let sddl = build_capability_pipe_sddl(None).expect("none path must succeed");
        assert_eq!(sddl, CAPABILITY_PIPE_SDDL);
    }

    /// `build_capability_pipe_sddl(Some(sid))` must embed the ACE before the
    /// SACL and preserve the `D:P` protected prefix + Low-IL SACL verbatim.
    #[test]
    fn build_capability_pipe_sddl_some_embeds_ace_before_sacl() {
        let sid = "S-1-5-117-1-2-3-4";
        let sddl = build_capability_pipe_sddl(Some(sid)).expect("valid sid must build");
        let expected = format!(
            "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;{sid})S:(ML;;NW;;;LW)"
        );
        assert_eq!(sddl, expected);
    }

    #[test]
    fn test_broker_file_handle_to_process_duplicates_handle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("broker.txt");
        std::fs::write(&path, b"hello").expect("write file");
        let file = File::open(&path).expect("open file");

        let grant =
            broker_file_handle_to_process(&file, BrokerTargetProcess::current(), AccessMode::Read)
                .expect("duplicate handle into current process");
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        let raw_handle = grant.raw_handle.expect("raw handle");
        assert_ne!(raw_handle, 0);

        unsafe {
            // SAFETY: The duplicated handle value came from `DuplicateHandle`
            // above and has not been wrapped or closed yet.
            windows_sys::Win32::Foundation::CloseHandle(raw_handle as usize as HANDLE);
        }
    }

    // Phase 18 AIPC-01 Task 3: per-fn unit tests for the new event/mutex
    // brokers. All gated `#[cfg(target_os = "windows")]` (the entire file is
    // already Windows-only via `#[path]` routing in `mod.rs` but we apply the
    // gate explicitly so the intent is unambiguous).

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_event_to_process_duplicates_handle() {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::CreateEventW;
        // SAFETY: anonymous event creation with NULL attributes/name. Manual
        // reset = FALSE, initial state = FALSE.
        let event: HANDLE = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
        assert!(
            !event.is_null(),
            "CreateEventW failed: {}",
            std::io::Error::last_os_error()
        );

        let grant = broker_event_to_process(
            event,
            BrokerTargetProcess::current(),
            crate::supervisor::policy::EVENT_DEFAULT_MASK,
        )
        .expect("duplicate event into current process");
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(
            grant.resource_kind,
            crate::supervisor::types::GrantedResourceKind::Event
        );
        let raw = grant.raw_handle.expect("raw handle present");
        assert_ne!(raw, 0);

        // SAFETY: `raw` came from DuplicateHandle just above; `event` came
        // from CreateEventW above. Both are live HANDLEs we own.
        unsafe {
            CloseHandle(raw as usize as HANDLE);
            CloseHandle(event);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_event_to_process_propagates_duplicate_handle_failure() {
        let result = broker_event_to_process(
            std::ptr::null_mut(),
            BrokerTargetProcess::current(),
            crate::supervisor::policy::EVENT_DEFAULT_MASK,
        );
        let err = result.expect_err("NULL source handle must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("DuplicateHandle (event"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_mutex_to_process_duplicates_handle() {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::CreateMutexW;
        // SAFETY: anonymous mutex creation with NULL attributes/name. Initial
        // owner = FALSE.
        let mutex: HANDLE = unsafe { CreateMutexW(std::ptr::null_mut(), 0, std::ptr::null()) };
        assert!(
            !mutex.is_null(),
            "CreateMutexW failed: {}",
            std::io::Error::last_os_error()
        );

        let grant = broker_mutex_to_process(
            mutex,
            BrokerTargetProcess::current(),
            crate::supervisor::policy::MUTEX_DEFAULT_MASK,
        )
        .expect("duplicate mutex into current process");
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(
            grant.resource_kind,
            crate::supervisor::types::GrantedResourceKind::Mutex
        );
        let raw = grant.raw_handle.expect("raw handle present");
        assert_ne!(raw, 0);

        // SAFETY: `raw` came from DuplicateHandle just above; `mutex` came
        // from CreateMutexW above. Both are live HANDLEs we own.
        unsafe {
            CloseHandle(raw as usize as HANDLE);
            CloseHandle(mutex);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_mutex_to_process_propagates_duplicate_handle_failure() {
        let result = broker_mutex_to_process(
            std::ptr::null_mut(),
            BrokerTargetProcess::current(),
            crate::supervisor::policy::MUTEX_DEFAULT_MASK,
        );
        let err = result.expect_err("NULL source handle must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("DuplicateHandle (mutex"),
            "unexpected error: {msg}"
        );
    }

    // Phase 18 AIPC-01 Plan 18-03 Task 1 — Job Object broker unit tests.
    // All gated `#[cfg(target_os = "windows")]`.

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_job_object_to_process_duplicates_with_query_mask() {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::CreateJobObjectW;
        // SAFETY: anonymous Job Object creation with NULL attributes/name.
        let job: HANDLE = unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
        assert!(
            !job.is_null(),
            "CreateJobObjectW failed: {}",
            std::io::Error::last_os_error()
        );

        let grant = broker_job_object_to_process(
            job,
            BrokerTargetProcess::current(),
            crate::supervisor::policy::JOB_OBJECT_DEFAULT_MASK,
        )
        .expect("duplicate Job Object into current process");
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(
            grant.resource_kind,
            crate::supervisor::types::GrantedResourceKind::JobObject
        );
        let raw = grant.raw_handle.expect("raw handle present");
        assert_ne!(raw, 0);

        // SAFETY: `raw` came from DuplicateHandle just above; `job` came from
        // CreateJobObjectW above. Both are live HANDLEs we own.
        unsafe {
            CloseHandle(raw as usize as HANDLE);
            CloseHandle(job);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_job_object_to_process_propagates_duplicate_handle_failure() {
        let result = broker_job_object_to_process(
            std::ptr::null_mut(),
            BrokerTargetProcess::current(),
            crate::supervisor::policy::JOB_OBJECT_DEFAULT_MASK,
        );
        let err = result.expect_err("NULL source handle must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("DuplicateHandle (Job Object,"),
            "unexpected error: {msg}"
        );
    }

    // Phase 18 AIPC-01 Plan 18-02 Task 2 — pipe + socket broker unit tests.
    // All gated `#[cfg(target_os = "windows")]` (the file is already
    // Windows-only via #[path] routing in mod.rs but the explicit gate makes
    // the intent unambiguous and survives any future routing change).

    fn unique_aipc_pipe_name(suffix: &str) -> String {
        // Use the per-process PID + a SHA-derived suffix to avoid collisions
        // between concurrent test invocations within the same process. The
        // canonical AIPC namespace prefix is `\\.\pipe\nono-aipc-<id>-<name>`.
        format!(r"\\.\pipe\nono-aipc-test{}-{suffix}", std::process::id())
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_pipe_to_process_duplicates_with_read_access() {
        use windows_sys::Win32::Foundation::CloseHandle;

        let name = unique_aipc_pipe_name("pipe-read");
        let handle = bind_aipc_pipe(&name, PipeDirection::Read).expect("bind aipc pipe");
        assert_ne!(handle, INVALID_HANDLE_VALUE);

        let grant =
            broker_pipe_to_process(handle, BrokerTargetProcess::current(), PipeDirection::Read)
                .expect("broker pipe with read direction");
        assert_eq!(
            grant.transfer,
            ResourceTransferKind::DuplicatedWindowsHandle
        );
        assert_eq!(
            grant.resource_kind,
            crate::supervisor::types::GrantedResourceKind::Pipe
        );
        assert_eq!(grant.access, AccessMode::Read);
        let raw = grant.raw_handle.expect("raw handle present");
        assert_ne!(raw, 0);

        // SAFETY: `raw` came from DuplicateHandle just above; `handle` came
        // from CreateNamedPipeW above. Both are live HANDLEs we own.
        unsafe {
            CloseHandle(raw as usize as HANDLE);
            CloseHandle(handle);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_pipe_to_process_duplicates_with_readwrite_access() {
        use windows_sys::Win32::Foundation::CloseHandle;

        let name = unique_aipc_pipe_name("pipe-rw");
        let handle = bind_aipc_pipe(&name, PipeDirection::ReadWrite).expect("bind aipc pipe");
        assert_ne!(handle, INVALID_HANDLE_VALUE);

        let grant = broker_pipe_to_process(
            handle,
            BrokerTargetProcess::current(),
            PipeDirection::ReadWrite,
        )
        .expect("broker pipe with read+write direction");
        assert_eq!(grant.access, AccessMode::ReadWrite);
        let raw = grant.raw_handle.expect("raw handle present");
        assert_ne!(raw, 0);

        unsafe {
            CloseHandle(raw as usize as HANDLE);
            CloseHandle(handle);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_pipe_to_process_propagates_duplicate_handle_failure() {
        let result = broker_pipe_to_process(
            std::ptr::null_mut(),
            BrokerTargetProcess::current(),
            PipeDirection::Read,
        );
        let err = result.expect_err("NULL source handle must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("DuplicateHandle (pipe,"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_socket_to_process_serializes_proto_info_blob() {
        use windows_sys::Win32::Networking::WinSock::{
            closesocket, WSASocketW, WSAStartup, AF_INET, IPPROTO_TCP, SOCK_STREAM, WSADATA,
            WSA_FLAG_OVERLAPPED,
        };

        // SAFETY: WSAStartup with version 2.2 (0x0202) is the standard
        // Winsock initialization; wsadata points to writable storage. The
        // call is reference-counted and idempotent.
        let mut wsadata: WSADATA = unsafe { std::mem::zeroed() };
        let _ = unsafe { WSAStartup(0x0202, &mut wsadata) };

        // SAFETY: WSASocketW with NULL protocol_info creates a fresh socket.
        // AF_INET / SOCK_STREAM / IPPROTO_TCP are well-defined constants.
        let sock = unsafe {
            WSASocketW(
                AF_INET as i32,
                SOCK_STREAM,
                IPPROTO_TCP,
                std::ptr::null(),
                0,
                WSA_FLAG_OVERLAPPED,
            )
        };
        assert_ne!(
            sock,
            INVALID_SOCKET,
            "WSASocketW failed: {}",
            std::io::Error::last_os_error()
        );

        let target_pid = unsafe { GetCurrentProcessId() };
        let grant = broker_socket_to_process(
            sock,
            BrokerTargetProcess::current(),
            target_pid,
            SocketRole::Connect,
        )
        .expect("broker socket should serialize proto_info blob");
        assert_eq!(grant.transfer, ResourceTransferKind::SocketProtocolInfoBlob);
        assert_eq!(
            grant.resource_kind,
            crate::supervisor::types::GrantedResourceKind::Socket
        );
        assert!(grant.raw_handle.is_none());
        let blob = grant.protocol_info_blob.as_ref().expect("blob present");
        assert_eq!(
            blob.len(),
            std::mem::size_of::<WSAPROTOCOL_INFOW>(),
            "blob length must match WSAPROTOCOL_INFOW size"
        );

        // SAFETY: `sock` is the live SOCKET we created via WSASocketW above
        // and have not freed elsewhere.
        unsafe {
            closesocket(sock);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_broker_socket_to_process_propagates_wsa_failure() {
        let result = broker_socket_to_process(
            INVALID_SOCKET,
            BrokerTargetProcess::current(),
            unsafe { GetCurrentProcessId() },
            SocketRole::Connect,
        );
        let err = result.expect_err("INVALID_SOCKET source must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("WSADuplicateSocketW failed"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_bind_aipc_pipe_creates_pipe_with_low_integrity_sddl() {
        use windows_sys::Win32::Foundation::CloseHandle;

        // The SDDL applied is the byte-identical Phase 11 CAPABILITY_PIPE_SDDL
        // constant via build_low_integrity_security_attributes(); the SDDL
        // contents themselves are exercised by test_bind_low_integrity_roundtrip
        // (Phase 11). Here we verify bind_aipc_pipe produces a usable handle.
        let name = unique_aipc_pipe_name("aipctest-sddl");
        let handle = bind_aipc_pipe(&name, PipeDirection::ReadWrite).expect("bind aipc pipe");
        assert_ne!(handle, INVALID_HANDLE_VALUE);
        assert!(!handle.is_null(), "handle should be non-NULL");

        // SAFETY: `handle` is a live HANDLE returned by bind_aipc_pipe above.
        unsafe {
            CloseHandle(handle);
        }
    }
}
