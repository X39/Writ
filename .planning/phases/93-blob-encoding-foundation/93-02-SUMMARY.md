---
phase: 93-blob-encoding-foundation
plan: 02
subsystem: compiler
tags: [writ-compiler, attribute-encoding, blob-heap, writ-module, attr]

# Dependency graph
requires:
  - phase: 93-01
    provides: AttrValue enum, encode_attr_args/decode_attr_args in writ-module/src/attr.rs
provides:
  - Real attribute argument encoding in collect_attributes (writ-compiler/src/emit/collect/encoding.rs)
  - map_attr_arg and map_attr_expr helpers for AST-to-AttrValue conversion
  - NULL-owner guard: attributes with no metadata token are skipped entirely
affects:
  - Phase 94 (Deprecated attribute semantic effects — reads blob heap offsets written here)
  - Phase 95 (Conditional attribute — reads/writes blob heap same way)
  - Runtime attribute query API phases

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "filter_map(map_attr_arg) pattern for lossy AST-to-wire-value conversion: unsupported expr types silently skipped"
    - "blob_offset=0 for empty arg lists, non-zero for encoded args — callers distinguish null blob from populated blob"
    - "None => continue guard on token_for_def replaces unwrap_or(NULL) to prevent NULL-owner AttributeDef rows"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/collect/encoding.rs

key-decisions:
  - "Unsupported AstExpr variants (floats, binary ops) are silently skipped via filter_map — no hard error, deferred to future phases"
  - "map_attr_arg and map_attr_expr are private free functions (not methods) — consistent with the functional style of the encoding module"

patterns-established:
  - "Attr encoding: collect args -> filter_map to AttrValue -> encode_attr_args -> blob_heap.intern -> pass offset to add_attribute_def"

requirements-completed: [BLOB-01, BLOB-02]

# Metrics
duration: 5min
completed: 2026-03-27
---

# Phase 93 Plan 02: Blob Encoding Foundation — Attribute Arg Encoding Summary

**collect_attributes now encodes string/int/bool/named attribute args into the blob heap, replacing the value=0 stub, with a NULL-owner skip guard**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-27T19:00:00Z
- **Completed:** 2026-03-27T19:03:34Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `map_attr_arg` and `map_attr_expr` helper functions to convert `AstAttributeArg` / `AstExpr` nodes to `AttrValue` instances
- Replaced the `value=0` hardcode with real encoding: non-empty arg lists are encoded via `encode_attr_args` and interned into `builder.blob_heap`
- Empty arg lists (no-arg attributes like `[Singleton]`) still produce blob offset 0 (null blob) — correct behavior preserved
- Fixed the NULL-owner guard: `unwrap_or(MetadataToken::NULL)` replaced with `None => continue` so attributes whose def has no metadata token are silently skipped rather than emitted with a NULL owner

## Task Commits

1. **Task 1: Replace value=0 stub with real encoding and fix NULL-owner guard** - `4259eee` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/collect/encoding.rs` - Added `use writ_module::attr::{AttrValue, encode_attr_args}` import, `map_attr_arg`/`map_attr_expr` helpers, and real encoding loop replacing the deferred-args stub

## Decisions Made

- Unsupported AstExpr variants (FloatLit, BinaryOp, etc.) are silently skipped via `filter_map` with a `None` arm in `map_attr_expr`. This matches the plan spec and keeps Phase 93 scope tight — float attribute args are uncommon in practice.
- Helper functions are private free functions (not methods or closures) to match the existing functional style of the encoding module.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Blob heap now contains real attribute argument data for any attribute with string/int/bool/named args
- Phase 94 (Deprecated attribute semantic effects) can read back blob data from AttributeDef rows via decode_attr_args
- Full workspace test suite: 0 failures across all crates

---
*Phase: 93-blob-encoding-foundation*
*Completed: 2026-03-27*

## Self-Check: PASSED

- `writ-compiler/src/emit/collect/encoding.rs` contains `use writ_module::attr::{AttrValue, encode_attr_args}` (line 6)
- `writ-compiler/src/emit/collect/encoding.rs` contains `fn map_attr_arg` (line 67)
- `writ-compiler/src/emit/collect/encoding.rs` contains `fn map_attr_expr` (line 80)
- `writ-compiler/src/emit/collect/encoding.rs` does NOT contain `"args encoding deferred"` comment
- `writ-compiler/src/emit/collect/encoding.rs` contains `None => continue` guard at line 118
- `writ-compiler/src/emit/collect/encoding.rs` contains `builder.blob_heap.intern(&bytes)` at line 135
- Commit `4259eee` verified: `git log --oneline | grep 4259eee` returns the commit
- All workspace tests: 0 failures
