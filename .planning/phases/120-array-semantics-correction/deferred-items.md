# Phase 120 Deferred Items

## Out-of-Scope Golden Test Failures

The following golden tests were already failing before Phase 120 execution began.
They are NOT caused by the array opcode changes. They are pre-existing failures
from Phases 117-118 that need re-blessing in a dedicated cleanup pass.

| Test | Last Blessed | Root Cause |
|------|-------------|------------|
| `golden_lib_preload_stub` | Phase 117-01 | Column offset drift in debug info |
| `golden_generic_inherent_impl` | Phase 118 | String offset change |
| `test_expr_string_escapes` | Phase 118 | String heap offset drift |
| `test_fn_overload` | Phase 118 | Opcode/offset change |
| `test_string_utilities` | Phase 118 | String offset drift |

**Action:** A future cleanup pass should re-bless these 5 golden files with
`BLESS=1 cargo test -p writ-golden`. Verify each diff is column/offset-only
(no semantic change) before accepting.
