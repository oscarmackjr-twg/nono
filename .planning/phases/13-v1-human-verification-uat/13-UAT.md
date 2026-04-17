# --- (YAML frontmatter start)
status: pending
phase: 13-v1-human-verification-uat
source:
  - .planning/phases/05-windows-detach-readiness-fix/05-VERIFICATION.md
  - .planning/phases/07-quick-wins/07-VERIFICATION.md
  - .planning/phases/09-wfp-port-level-proxy-filtering/09-VERIFICATION.md
  - .planning/phases/11-runtime-capability-expansion/11-VERIFICATION.md
host:
  windows_build: ""
  admin: ""
  nono_binary_commit: ""
  wfp_service_running: ""
started: ""
updated: ""
# --- (YAML frontmatter end)

# Phase 13 — v1.0 Human Verification UAT Runbook

This runbook closes the 10 deferred `human_needed` items from phases 05, 07, 09,
and 11. A human tester executes the items on a real Windows host in the
prescribed wave order, records a verdict for each, and then a follow-up plan
promotes the upstream `VERIFICATION.md` files.

Do **not** run Wave 2 before Wave 1, and do not skip the pre-flight checklist:
several items depend on earlier state (a live session, an admin shell, or a
running WFP service).

---

## Pre-flight Checklist

Complete these steps in order. Fill in the `host:` block in the YAML
frontmatter above from the recorded values.

1. **Windows build:** Run `winver` from the Run dialog or command prompt.
   - Expected: Windows 10 build **17763 or newer** (ConPTY + WFP requirement).
   - Record the build number into `host.windows_build`.

2. **Build nono from source:** From the repo root, run
   `cargo build -p nono-cli --release`.
   - Expected: Build succeeds with no errors. Binary at
     `target\release\nono.exe`.
   - Then run `git rev-parse --short HEAD` and record the commit hash into
     `host.nono_binary_commit`.

3. **Binary smoke test:** Run `target\release\nono.exe --version`.
   - Expected: Prints a version string like `nono 0.x.y`. No panic, no error.

4. **Admin status:** Run `net session`.
   - If the command returns a session list or `There are no entries in the
     list.` without an "Access is denied" error, the shell is elevated.
   - If it errors with `Access is denied.`, the shell is **not** elevated.
   - Record `yes` or `no` into `host.admin`. (Several items require `yes`.)

5. **WFP service:** Run `sc query nono-wfp-service`.
   - Expected for P09 items: `STATE : 4 RUNNING`.
   - If the service is `STOPPED` or does not exist, install/start it per the
     Phase 09 setup instructions before running P09-HV-1 / P09-HV-2.
   - Record `yes` or `no` into `host.wfp_service_running`.

6. **Fill in host frontmatter:** Open this file, set
   `host.windows_build`, `host.admin`, `host.nono_binary_commit`, and
   `host.wfp_service_running` to the values recorded in steps 1-5. Set
   `started` to the current UTC ISO-8601 timestamp.

---

## Tests

Tests are grouped into four dependency waves. Run waves in order. Within a
wave, any order is acceptable.

### Wave 1 — No dependencies (run first)

#### 1. P05-HV-1: Detach/Attach lifecycle
- command: `nono run --detach -- ping -t 127.0.0.1`
- expected: Exits within ~2 seconds with "Started detached session" banner.
  Record the session ID from output.
- then: `nono attach <session-id-from-above>`
- expected: Connects to running session, shows ping output. Ctrl-C to detach.
- result: [pending]

#### 2. P07-HV-2: Setup help text
- command: `nono setup --check-only`
- expected: Output contains the string `'nono wrap' is available on Windows
  with Job Object + WFP enforcement (no exec-replace, unlike Unix)` and does
  NOT contain `remain intentionally unavailable`.
- result: [pending]

#### 3. P09-HV-2: WFP port integration test
- prereq: Admin confirmed (pre-flight step 4), WFP service running (pre-flight
  step 5, `nono-wfp-service` in RUNNING state). If either is no, mark
  `blocked`.
- command: `cargo test -p nono-cli --test wfp_port_integration -- --ignored`
- expected: `wfp_port_permit_allows_real_tcp_connection` passes — TCP connect
  to ephemeral allowed port succeeds, TCP connect to blocked port fails. Test
  exits 0.
- result: [pending]

### Wave 2 — Depends on P05-HV-1 creating a session

#### 4. P07-HV-3: Session commands round-trip
- prereq: P05-HV-1 must have completed (creates a session record).
- commands (run each, record output):
  - `nono logs <session-id>` — shows session event log entries
  - `nono inspect <session-id>` — shows JSON session record
  - `nono prune --dry-run` — lists sessions eligible for pruning without
    deleting
- expected: Each reads from `~/.config/nono/sessions/` and returns real data.
  No `UnsupportedPlatform` error.
- result: [pending]

### Wave 3 — Independent items

#### 5. P07-HV-1: Wrap exit code propagation
- command: `nono wrap -- cmd.exe /c exit 42`
- then: `echo %ERRORLEVEL%` (cmd.exe) or `$LASTEXITCODE` (PowerShell)
- expected: Output is `42`. No panic, no `unreachable!()` error.
- result: [pending]

#### 6. P09-HV-1: Proxy env var injection
- prereq: Admin confirmed, WFP service (`nono-wfp-service`) running, a network
  profile with proxy credentials configured. If proxy config is not available,
  use a minimal test: start a TCP listener on any port (e.g.,
  `python -m http.server 8888`) and configure nono to use it as the proxy
  endpoint.
- command: `nono run --proxy-only -- cmd.exe /c set` (or PowerShell:
  `nono run --proxy-only -- powershell -c "Get-ChildItem Env:"`)
- expected: Child environment output contains
  `HTTPS_PROXY=http://localhost:<port>` and `NONO_PROXY_TOKEN=<token>`.
- result: [pending]
- note: If proxy configuration is not available or too complex to set up, mark
  `blocked` with reason.

### Wave 4 — Complex setup items (run last)

#### 7. P11-HV-1: End-to-end capability request with interactive prompt
- prereq: Built nono binary with Phase 11 code. No admin required.
- setup: The test requires a child process that speaks the capability pipe
  protocol. Create a PowerShell script `test-cap-client.ps1` with the content
  below. CRITICAL details:
  - Uses 4-byte big-endian length-prefixed framing, NOT newline-delimited.
  - `NONO_SUPERVISOR_PIPE` is a rendezvous FILE path (e.g.
    `C:\Users\<you>\AppData\Local\Temp\nono-cap-<sid>.pipe`), not a pipe name.
    The script must read the file and use line 1 as the pipe name.
  - Payload is a `SupervisorMessage::Request(CapabilityRequest)` enum, which
    serde serializes as `{"Request": { ...flat fields... }}`. There is NO
    `capability.fs` nesting.
  - `CapabilityRequest` fields are flat: `request_id`, `path`, `access`
    (PascalCase string), `reason`, `child_pid`, `session_id`, `session_token`.

```powershell
# test-cap-client.ps1 — Capability pipe test client
# Reads NONO_SESSION_TOKEN and NONO_SUPERVISOR_PIPE from env (set by nono supervisor)
$token = $env:NONO_SESSION_TOKEN
$rendezvousPath = $env:NONO_SUPERVISOR_PIPE

if (-not $token -or -not $rendezvousPath) {
    Write-Error "NONO_SESSION_TOKEN or NONO_SUPERVISOR_PIPE not set. Run inside nono supervised session."
    exit 1
}

# NONO_SUPERVISOR_PIPE is a rendezvous file on disk, NOT a \\.\pipe\ path.
# Contents: line 1 = pipe name (e.g. \\.\pipe\nono-cap-<sid>), line 2 = server PID.
if (-not (Test-Path $rendezvousPath)) {
    Write-Error "Rendezvous file not found at $rendezvousPath"
    exit 1
}
$pipeName = (Get-Content $rendezvousPath -TotalCount 1).Trim()
if (-not $pipeName.StartsWith('\\.\pipe\')) {
    Write-Error "Rendezvous line 1 is not a valid pipe name: $pipeName"
    exit 1
}

# Strip \\.\pipe\ prefix for NamedPipeClientStream
$shortName = $pipeName -replace '^\\\\\.\\pipe\\', ''

$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", $shortName, [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect(5000)

# Build SupervisorMessage::Request(CapabilityRequest) — flat fields wrapped in Request envelope
$envelope = @{
    Request = @{
        request_id    = [guid]::NewGuid().ToString()
        path          = 'C:\Temp'
        access        = 'Read'
        reason        = 'P11-HV-1 UAT probe'
        child_pid     = $PID
        session_id    = 'uat'
        session_token = $token
    }
} | ConvertTo-Json -Depth 4 -Compress
$payload = [System.Text.Encoding]::UTF8.GetBytes($envelope)

# Write 4-byte big-endian length prefix + payload
$lenBytes = [BitConverter]::GetBytes([uint32]$payload.Length)
if ([BitConverter]::IsLittleEndian) { [Array]::Reverse($lenBytes) }
$pipe.Write($lenBytes, 0, 4)
$pipe.Write($payload, 0, $payload.Length)
$pipe.Flush()

# Read 4-byte length prefix response
$respLenBytes = New-Object byte[] 4
$pipe.Read($respLenBytes, 0, 4) | Out-Null
if ([BitConverter]::IsLittleEndian) { [Array]::Reverse($respLenBytes) }
$respLen = [BitConverter]::ToUInt32($respLenBytes, 0)

# Read response payload
$respPayload = New-Object byte[] $respLen
$pipe.Read($respPayload, 0, $respLen) | Out-Null
$response = [System.Text.Encoding]::UTF8.GetString($respPayload)
Write-Output "Response: $response"

$pipe.Dispose()
```

- command: `nono run --supervised -- powershell -ExecutionPolicy Bypass -File
  test-cap-client.ps1`
- expected: The supervisor console displays `[nono] Grant access? [y/N]`
  prompt. Replying `y` returns a grant response to the child. Replying `N`
  returns a Denied response.
- result: [pending]

#### 8. P11-HV-3: Token leak audit under RUST_LOG=trace
- prereq: Same supervised session setup as P11-HV-1, or run a fresh one.
- setup: Before running, note the session token value (it will appear in the
  child's environment).
- command: `set RUST_LOG=trace` then `nono run --supervised -- powershell
  -ExecutionPolicy Bypass -File test-cap-client.ps1 2> trace-output.txt`
- then: `findstr /C:"<full-64-char-hex-token-value>" trace-output.txt`
- expected: `findstr` returns no matches (exit code 1). Zero log lines contain
  the token value.
- IMPORTANT: Use the full 64-character hex token value in the findstr pattern,
  not a substring. A substring could match unrelated hex strings in the log.
- cleanup: After verification, delete `trace-output.txt` — it may contain
  sensitive debug data (T-13-01).
- result: [pending]

#### 9. P11-HV-2: Low Integrity child pipe connectivity (WAIVER CANDIDATE)
- rationale for waiver: Spawning a child at Low Integrity on Windows requires
  `WinSaferCreateLevel` + `SaferComputeTokenFromLevel` or duplicating a
  process token and setting its integrity label. This is non-trivial to do
  interactively and not achievable through normal CLI use. The SDDL
  `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;LW)` on the pipe is
  verified by code inspection in 11-VERIFICATION.md, and the
  `test_bind_low_integrity_roundtrip` unit test exercises bind/connect within
  a single process. Cross-boundary LI access requires specialized tooling
  outside the UAT scope.
- recommendation: Waive with documented SDDL evidence. Mark `waived`.
- result: [pending]

#### 10. P05-HV-2: WaitNamedPipeW fail-closed path (WAIVER CANDIDATE)
- rationale for waiver: Triggering a specific Windows API error code (other
  than `ERROR_FILE_NOT_FOUND` or `ERROR_SEM_TIMEOUT`) from `WaitNamedPipeW`
  without process manipulation or a custom debug build is impractical. The
  fail-closed code path is verified by code inspection in 05-VERIFICATION.md
  Truth #3: lines 88-93 of `startup_runtime.rs` return
  `Err(NonoError::SandboxInit(...))` for any unexpected error. No reasonable
  production scenario triggers this path without injecting faults.
- recommendation: Waive with documented code-inspection evidence. Mark
  `waived`.
- result: [pending]

---

## Summary

```
total: 10
passed: 0
issues: 0
pending: 10
skipped: 0
blocked: 0
waived: 0
```

---

## Gaps

[none — populate this section if any item fails]

---

## Outcome Handling

| Result  | Action                                                         |
|---------|----------------------------------------------------------------|
| pass    | Update upstream VERIFICATION.md; close item                    |
| fail    | File gap in Gaps section; keep upstream at human_needed        |
| waived  | Document rationale; update upstream with waived note           |
| blocked | Document blocker; do NOT update upstream                       |
