//! Integration test: end-to-end AIPC-01 (Phase 18) broker round-trip for
//! all 5 new handle types.
//!
//! Exercises Event, Mutex, Pipe, Socket, Job Object per-kind round-trips
//! through the public `nono::supervisor::socket` broker functions with
//! `BrokerTargetProcess::current()` so duplication targets the test
//! process itself — no admin required (unlike `wfp_port_integration.rs`,
//! which exercises the WFP service).
//!
//! Each test creates the supervisor-side source kernel object, calls
//! the corresponding `broker_*_to_process` function, asserts the
//! returned `ResourceGrant` matches the expected shape (resource_kind,
//! transfer mechanism, raw_handle / protocol_info_blob presence), and
//! closes the duplicated handle to avoid kernel-object leaks.
//!
//! Mirrors the in-source `socket_windows::tests::test_broker_*`
//! suite — but in a SEPARATE test binary, which catches build / link
//! issues that the in-source tests would not.
//!
//! # Running
//!
//!   cargo test -p nono-cli --test aipc_handle_brokering_integration
//!
//! Cross-platform compile: this file is `#[cfg(target_os = "windows")]`-
//! gated, so it produces an empty test binary on Linux/macOS; CI
//! `cargo build -p nono-cli --tests` still passes everywhere.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use nono::supervisor::policy;
use nono::supervisor::{
    GrantedResourceKind, PipeDirection, ResourceTransferKind, SocketRole,
};
use nono::BrokerTargetProcess;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

#[test]
fn integration_event_broker_round_trip() {
    use windows_sys::Win32::System::Threading::CreateEventW;
    // SAFETY: anonymous event creation with NULL attributes/name. Manual
    // reset = FALSE, initial state = FALSE.
    let source: HANDLE =
        unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
    assert!(
        !source.is_null(),
        "CreateEventW failed: {}",
        std::io::Error::last_os_error()
    );

    let grant = nono::supervisor::socket::broker_event_to_process(
        source,
        BrokerTargetProcess::current(),
        policy::EVENT_DEFAULT_MASK,
    )
    .expect("broker event into current process");
    assert_eq!(grant.resource_kind, GrantedResourceKind::Event);
    assert_eq!(grant.transfer, ResourceTransferKind::DuplicatedWindowsHandle);
    let dup = grant.raw_handle.expect("event grant must carry raw handle");
    assert_ne!(dup, 0);

    // SAFETY: both HANDLEs are live; dup came from DuplicateHandle
    // inside the broker, source came from CreateEventW above.
    unsafe {
        CloseHandle(dup as usize as HANDLE);
        CloseHandle(source);
    }
}

#[test]
fn integration_mutex_broker_round_trip() {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    // SAFETY: anonymous mutex creation with NULL attrs/name; initial
    // owner = FALSE.
    let source: HANDLE =
        unsafe { CreateMutexW(std::ptr::null_mut(), 0, std::ptr::null()) };
    assert!(
        !source.is_null(),
        "CreateMutexW failed: {}",
        std::io::Error::last_os_error()
    );

    let grant = nono::supervisor::socket::broker_mutex_to_process(
        source,
        BrokerTargetProcess::current(),
        policy::MUTEX_DEFAULT_MASK,
    )
    .expect("broker mutex into current process");
    assert_eq!(grant.resource_kind, GrantedResourceKind::Mutex);
    assert_eq!(grant.transfer, ResourceTransferKind::DuplicatedWindowsHandle);
    let dup = grant.raw_handle.expect("mutex grant must carry raw handle");
    assert_ne!(dup, 0);

    // SAFETY: both HANDLEs are live; see broker docs.
    unsafe {
        CloseHandle(dup as usize as HANDLE);
        CloseHandle(source);
    }
}

#[test]
fn integration_pipe_broker_round_trip() {
    // bind_aipc_pipe is the supervisor-side helper that creates the named
    // pipe with the byte-identical Phase 11 SDDL via
    // build_low_integrity_security_attributes. The integration test
    // exercises the full lifecycle (CreateNamedPipeW + DuplicateHandle).
    let canonical = format!(
        "\\\\.\\pipe\\nono-aipc-integ-{}-pipe",
        std::process::id()
    );
    let source = nono::supervisor::socket::bind_aipc_pipe(&canonical, PipeDirection::Read)
        .expect("bind AIPC pipe");

    let grant = nono::supervisor::socket::broker_pipe_to_process(
        source,
        BrokerTargetProcess::current(),
        PipeDirection::Read,
    )
    .expect("broker pipe into current process");
    assert_eq!(grant.resource_kind, GrantedResourceKind::Pipe);
    assert_eq!(grant.transfer, ResourceTransferKind::DuplicatedWindowsHandle);
    assert_eq!(grant.access, nono::AccessMode::Read);
    let dup = grant.raw_handle.expect("pipe grant must carry raw handle");
    assert_ne!(dup, 0);

    // SAFETY: both HANDLEs are live; bind_aipc_pipe returned source,
    // DuplicateHandle returned dup.
    unsafe {
        CloseHandle(dup as usize as HANDLE);
        CloseHandle(source);
    }
}

#[test]
fn integration_socket_broker_round_trip() {
    use windows_sys::Win32::Networking::WinSock::{
        closesocket, WSASocketW, WSAStartup, AF_INET, INVALID_SOCKET, IPPROTO_TCP,
        SOCK_STREAM, WSADATA, WSA_FLAG_OVERLAPPED,
    };
    // SAFETY: WSAStartup with version 2.2 (0x0202); reference-counted
    // initialization, idempotent for the lifetime of the process.
    let mut wsa: WSADATA = unsafe { std::mem::zeroed() };
    let _ = unsafe { WSAStartup(0x0202, &mut wsa) };

    // SAFETY: WSASocketW with NULL protocol_info creates a fresh socket.
    // AF_INET / SOCK_STREAM / IPPROTO_TCP are well-defined Winsock
    // constants; WSA_FLAG_OVERLAPPED is the standard flag for
    // brokered sockets.
    let source = unsafe {
        WSASocketW(
            AF_INET as i32,
            SOCK_STREAM,
            IPPROTO_TCP,
            std::ptr::null(),
            0,
            WSA_FLAG_OVERLAPPED,
        )
    };
    assert_ne!(source, INVALID_SOCKET, "WSASocketW must succeed");

    let grant = nono::supervisor::socket::broker_socket_to_process(
        source,
        BrokerTargetProcess::current(),
        std::process::id(),
        SocketRole::Connect,
    )
    .expect("broker socket into current process");
    assert_eq!(grant.resource_kind, GrantedResourceKind::Socket);
    assert_eq!(
        grant.transfer,
        ResourceTransferKind::SocketProtocolInfoBlob
    );
    assert!(
        grant.raw_handle.is_none(),
        "socket grant uses protocol_info_blob, not raw_handle"
    );
    let blob = grant
        .protocol_info_blob
        .as_ref()
        .expect("socket grant must carry protocol_info_blob");
    assert_eq!(
        blob.len(),
        std::mem::size_of::<windows_sys::Win32::Networking::WinSock::WSAPROTOCOL_INFOW>(),
        "blob length must match WSAPROTOCOL_INFOW size"
    );

    // SAFETY: source SOCKET is live; closesocket returns 0 on success
    // and -1 on failure (the `_` discards the return for the
    // post-broker close per CONTEXT.md D-10).
    let _ = unsafe { closesocket(source) };
}

#[test]
fn integration_job_object_broker_round_trip() {
    use windows_sys::Win32::System::JobObjects::CreateJobObjectW;
    // SAFETY: anonymous Job Object creation with NULL attributes/name.
    let source: HANDLE =
        unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
    assert!(
        !source.is_null(),
        "CreateJobObjectW failed: {}",
        std::io::Error::last_os_error()
    );

    let grant = nono::supervisor::socket::broker_job_object_to_process(
        source,
        BrokerTargetProcess::current(),
        policy::JOB_OBJECT_DEFAULT_MASK,
    )
    .expect("broker Job Object into current process");
    assert_eq!(grant.resource_kind, GrantedResourceKind::JobObject);
    assert_eq!(grant.transfer, ResourceTransferKind::DuplicatedWindowsHandle);
    let dup = grant
        .raw_handle
        .expect("Job Object grant must carry raw handle");
    assert_ne!(dup, 0);

    // SAFETY: both HANDLEs are live; dup came from DuplicateHandle inside
    // the broker, source came from CreateJobObjectW above.
    unsafe {
        CloseHandle(dup as usize as HANDLE);
        CloseHandle(source);
    }
}
