//! Per-handle-type access-mask allowlists for AIPC-01 (Phase 18).
//!
//! All Win32 mask values are documented at learn.microsoft.com:
//!   - Sync objects: <https://learn.microsoft.com/en-us/windows/win32/sync/synchronization-object-security-and-access-rights>
//!   - Job Objects:  <https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights>
//!   - Generic:      <https://learn.microsoft.com/en-us/windows/win32/secauthz/generic-access-rights>
//!
//! Hard-coded defaults per CONTEXT.md D-05; profile widening (D-06) is layered
//! on top by the CLI in Plan 18-03.

use crate::supervisor::types::HandleKind;

// Standard access rights (shared)
pub const SYNCHRONIZE: u32 = 0x0010_0000;
pub const DELETE: u32 = 0x0001_0000;
pub const READ_CONTROL: u32 = 0x0002_0000;
pub const WRITE_DAC: u32 = 0x0004_0000;
pub const WRITE_OWNER: u32 = 0x0008_0000;

// Generic rights (file/pipe direction mapping)
pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const GENERIC_EXECUTE: u32 = 0x2000_0000;
pub const GENERIC_ALL: u32 = 0x1000_0000;

// Job Object specific
pub const JOB_OBJECT_ASSIGN_PROCESS: u32 = 0x0001;
pub const JOB_OBJECT_SET_ATTRIBUTES: u32 = 0x0002;
pub const JOB_OBJECT_QUERY: u32 = 0x0004;
pub const JOB_OBJECT_TERMINATE: u32 = 0x0008;
pub const JOB_OBJECT_SET_SECURITY_ATTRIBUTES: u32 = 0x0010; // Vista+: not supported
pub const JOB_OBJECT_ALL_ACCESS: u32 = 0x1F_001F;

// Event specific
pub const EVENT_MODIFY_STATE: u32 = 0x0002;
pub const EVENT_ALL_ACCESS: u32 = 0x1F_0003;

// Mutex specific
//
// NOTE: `MUTEX_MODIFY_STATE` is documented by Microsoft as "Reserved for
// future use" in the synchronization-object access-rights table. Calling
// `ReleaseMutex` works against handles opened with `SYNCHRONIZE` alone in
// current Windows. The bit is included here for SYMMETRY with
// `EVENT_MODIFY_STATE` and forward-compat (in case Microsoft activates the
// access right). Do NOT strip this constant — it is intentional and
// documented as a no-op-today, may-not-be-tomorrow signal.
pub const MUTEX_MODIFY_STATE: u32 = 0x0001;
pub const MUTEX_ALL_ACCESS: u32 = 0x1F_0001;

// Per-CONTEXT.md D-05 hard-coded defaults:
pub const JOB_OBJECT_DEFAULT_MASK: u32 = JOB_OBJECT_QUERY; // 0x0004
pub const EVENT_DEFAULT_MASK: u32 = SYNCHRONIZE | EVENT_MODIFY_STATE; // 0x0010_0002
pub const MUTEX_DEFAULT_MASK: u32 = SYNCHRONIZE | MUTEX_MODIFY_STATE; // 0x0010_0001

/// Maximum port number considered privileged. Ports `<= PRIVILEGED_PORT_MAX`
/// are unconditionally denied for Socket bind/listen requests regardless of
/// profile widening (CONTEXT.md `<specifics>` line 167; enforced at request
/// validation time in Plan 18-02).
pub const PRIVILEGED_PORT_MAX: u16 = 1023;

/// Validate a requested mask against the resolved per-type allowlist.
///
/// `requested` is the mask from the inbound `CapabilityRequest.access_mask`
/// (UNTRUSTED — client-declared). `resolved` is the supervisor-side computed
/// allowlist for this handle kind: hard-coded default ∪ profile widening.
///
/// Returns `true` iff every set bit in `requested` is also set in `resolved`
/// (subset semantic — D-07 server-side enforcement).
///
/// `_kind` is reserved for forward-compat per-kind dispatch; current
/// implementation is a single bitmask subset check.
#[must_use]
pub fn mask_is_allowed(_kind: HandleKind, requested: u32, resolved: u32) -> bool {
    requested & !resolved == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_subset_validates_correctly() {
        assert!(mask_is_allowed(HandleKind::JobObject, 0x0004, 0x0004));
        assert!(!mask_is_allowed(HandleKind::JobObject, 0x0008, 0x0004));
        assert!(mask_is_allowed(
            HandleKind::Event,
            0x0010_0002,
            0x0010_0002
        ));
        assert!(!mask_is_allowed(
            HandleKind::Event,
            EVENT_ALL_ACCESS,
            EVENT_DEFAULT_MASK
        ));
        assert!(mask_is_allowed(
            HandleKind::Mutex,
            MUTEX_DEFAULT_MASK,
            MUTEX_DEFAULT_MASK
        ));
        assert!(!mask_is_allowed(
            HandleKind::Mutex,
            MUTEX_ALL_ACCESS,
            MUTEX_DEFAULT_MASK
        ));
    }

    #[test]
    fn empty_request_is_trivially_subset() {
        assert!(mask_is_allowed(HandleKind::JobObject, 0, 0));
        assert!(mask_is_allowed(
            HandleKind::JobObject,
            0,
            JOB_OBJECT_DEFAULT_MASK
        ));
        assert!(mask_is_allowed(HandleKind::Event, 0, EVENT_DEFAULT_MASK));
    }

    #[test]
    fn all_bits_request_always_denied_unless_resolved_is_all_bits() {
        assert!(!mask_is_allowed(
            HandleKind::Event,
            0xFFFF_FFFF,
            EVENT_DEFAULT_MASK
        ));
        assert!(!mask_is_allowed(
            HandleKind::Mutex,
            0xFFFF_FFFF,
            MUTEX_DEFAULT_MASK
        ));
        assert!(!mask_is_allowed(
            HandleKind::JobObject,
            0xFFFF_FFFF,
            JOB_OBJECT_DEFAULT_MASK
        ));
        assert!(mask_is_allowed(
            HandleKind::JobObject,
            0xFFFF_FFFF,
            0xFFFF_FFFF
        ));
    }

    #[test]
    fn default_masks_match_d05_lock() {
        assert_eq!(JOB_OBJECT_DEFAULT_MASK, 0x0004);
        assert_eq!(EVENT_DEFAULT_MASK, 0x0010_0002);
        assert_eq!(MUTEX_DEFAULT_MASK, 0x0010_0001);
        assert_eq!(PRIVILEGED_PORT_MAX, 1023);
    }

    #[test]
    fn mutex_modify_state_documented_as_reserved() {
        // MUTEX_MODIFY_STATE is included in MUTEX_DEFAULT_MASK for SYMMETRY
        // with EVENT_MODIFY_STATE and forward-compat. Microsoft documents the
        // bit as "Reserved for future use"; ReleaseMutex works against
        // SYNCHRONIZE alone in current Windows. This test guards the
        // intentional inclusion against a future "dead code" cleanup.
        assert_eq!(MUTEX_MODIFY_STATE, 0x0001);
        // Compile-time assertion: MUTEX_DEFAULT_MASK must contain
        // MUTEX_MODIFY_STATE. Using `const _` because both operands are
        // constants and `assert!` on a constant value would be a clippy lint
        // (`assertions_on_constants`).
        const _: () = assert!(
            MUTEX_DEFAULT_MASK & MUTEX_MODIFY_STATE == MUTEX_MODIFY_STATE,
            "MUTEX_DEFAULT_MASK must include MUTEX_MODIFY_STATE for symmetry with EVENT_MODIFY_STATE"
        );
    }

    #[test]
    fn standard_constants_match_microsoft_table() {
        // Defense against an accidental constant edit. These are pinned by
        // the Microsoft Win32 API contract and must not drift.
        assert_eq!(SYNCHRONIZE, 0x0010_0000);
        assert_eq!(GENERIC_READ, 0x8000_0000);
        assert_eq!(GENERIC_WRITE, 0x4000_0000);
        assert_eq!(EVENT_MODIFY_STATE, 0x0002);
        assert_eq!(JOB_OBJECT_QUERY, 0x0004);
        assert_eq!(JOB_OBJECT_TERMINATE, 0x0008);
    }
}
