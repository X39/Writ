---
phase: 93-blob-encoding-foundation
plan: 01
subsystem: binary-module-format
tags: [writ-module, blob-encoding, attribute-args, binary-format, round-trip]

# Dependency graph
requires: []
provides:
  - AttrValue enum (String, Int, Bool, Named variants) in writ-module/src/attr.rs
  - ATTR_TAG_{STRING,INT,BOOL,NAMED} constants (0x01-0x04) as pub const in writ-module
  - encode_attr_args(&[AttrValue]) -> Vec<u8> free function in writ-module
  - decode_attr_args(&[u8]) -> Result<Vec<AttrValue>, DecodeError> free function in writ-module
  - InvalidAttrTag(u8) variant added to DecodeError in writ-module/src/error.rs
  - 7 round-trip tests in writ-module/tests/attr_encoding.rs
affects:
  - 93-02-compiler-encoding (imports AttrValue and encode_attr_args to replace value=0 stub)
  - writ-runtime (imports decode_attr_args to read attribute args from loaded modules)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tagged binary encoding: 1-byte tag prefix + typed payload, linear concatenation for multi-arg blobs"
    - "TDD pair: write failing test file first (RED), then implement (GREEN), commit together"
    - "Encoder/decoder co-located in writ-module so both compiler and runtime share the same constants"

key-files:
  created:
    - writ-module/src/attr.rs
    - writ-module/tests/attr_encoding.rs
  modified:
    - writ-module/src/error.rs
    - writ-module/src/lib.rs

key-decisions:
  - "encode_attr_args returns empty Vec for empty slice — caller uses blob offset 0 (null blob), not a zero-length interned blob"
  - "No new dependencies: manual LE byte array arithmetic replaces byteorder crate for simplicity"
  - "AttrValue::String(String) uses std::string::String to avoid ambiguity with the enum variant name"

patterns-established:
  - "Pattern: Tagged attr blob format — ATTR_TAG_STRING(0x01)/INT(0x02)/BOOL(0x03)/NAMED(0x04) with typed payloads"
  - "Pattern: decode_one returns (AttrValue, consumed_bytes) for cursor-based linear walking"

requirements-completed: [BLOB-01, BLOB-02, BLOB-03]

# Metrics
duration: 5min
completed: 2026-03-27
---

# Phase 93 Plan 01: Blob Encoding Foundation Summary

**Tagged binary attribute arg encoding in writ-module with AttrValue enum, ATTR_TAG_* constants, encode/decode functions, and 7 passing round-trip tests**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-27T18:52:06Z
- **Completed:** 2026-03-27T18:56:43Z
- **Tasks:** 2 (TDD pair — test file + implementation, committed together)
- **Files modified:** 4

## Accomplishments

- Created `writ-module/src/attr.rs` with public AttrValue enum (4 variants), 4 tag constants, encode_attr_args, and decode_attr_args
- Added `InvalidAttrTag(u8)` to `DecodeError` for unknown tag bytes
- Exposed `pub mod attr` and `pub use attr::AttrValue` from `writ-module/src/lib.rs`
- 7 tests in `attr_encoding.rs` pass: empty args, string, int (42/MAX/MIN), bool (true/false), named, multi-arg, invalid tag error

## Task Commits

Each task was committed atomically:

1. **Task 1+2 (TDD pair): Create attr.rs + attr_encoding.rs tests** - `47fb208` (feat)

**Plan metadata:** (docs commit follows)

_Note: Tasks 1 and 2 are a TDD pair — test file written first (RED confirmed), then implementation (GREEN confirmed). Both committed in a single atomic commit per the plan's TDD note._

## Files Created/Modified

- `writ-module/src/attr.rs` - AttrValue enum, ATTR_TAG_* constants, encode_attr_args, decode_attr_args, decode_one (internal)
- `writ-module/tests/attr_encoding.rs` - 7 round-trip tests covering all AttrValue variants and error case
- `writ-module/src/error.rs` - Added InvalidAttrTag(u8) variant to DecodeError
- `writ-module/src/lib.rs` - Added `pub mod attr;` and `pub use attr::AttrValue;`

## Decisions Made

- Empty slice encodes to empty Vec — callers should use blob offset 0 (null blob) for no-arg attributes rather than interning a zero-length blob. This matches the existing convention documented in the research.
- No byteorder crate usage in attr.rs — manual `to_le_bytes()` / `from_le_bytes()` is cleaner for small fixed-size reads; byteorder is still used in heap.rs where it was already present.
- `AttrValue::String(std::string::String)` — qualified to avoid name ambiguity between the enum variant `String` and the type `String` within the same module.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Known Stubs

None — all encode/decode paths are fully implemented for all 4 tag types.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `writ-module` now exports the full blob encoding contract (BLOB-01, BLOB-02, BLOB-03 complete)
- Plan 93-02 can import `AttrValue` and `encode_attr_args` from `writ-module` to replace the `value=0` stub in `writ-compiler/src/emit/collect/encoding.rs`
- `writ-runtime` can import `decode_attr_args` to read attribute argument blobs from loaded `.writc` modules

---
*Phase: 93-blob-encoding-foundation*
*Completed: 2026-03-27*
