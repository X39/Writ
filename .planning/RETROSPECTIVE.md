# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — CST-to-AST Lowering Pipeline

**Shipped:** 2026-02-27
**Phases:** 7 | **Plans:** 13 | **Commits:** 67

### What Was Built
- Complete AST type hierarchy (AstExpr/AstStmt/AstDecl/AstType) with span preservation and error recovery
- Multi-pass lowering pipeline: optional sugar, formattable strings, compound assignments, operators, dialogue, entities
- Dialogue lowering with three-tier speaker resolution, FNV-1a localization keys, choice branch scoping
- Entity lowering with component field flattening, lifecycle hooks, [Singleton] propagation
- 69 insta snapshot tests locking down every lowering pass plus integration/determinism checks
- Clean pipeline infrastructure: LoweringContext with error accumulation, pass ordering, public API

### What Worked
- **Dependency-ordered phase sequencing** — building AST types first, then expression helpers, then structural passes meant zero rework and shared helpers flowed naturally
- **Snapshot tests from Phase 2 onward** — every new pass was tested immediately; Phase 6 integration tests required minimal new work because per-pass coverage was already comprehensive
- **Audit → gap closure cycle** — the milestone audit identified three real integration issues (speaker stack leak, dead code, stale docs) that Phase 7 closed surgically; without the audit these would have shipped as latent bugs
- **Manual fold pattern** — simple and direct; no visitor framework overhead for 7 passes; each lowering file is self-contained

### What Was Inefficient
- **SUMMARY.md frontmatter one-liners never populated** — all 13 summaries have empty `requirements_completed` and null `one_liner` fields; the milestone completion tooling couldn't auto-extract accomplishments
- **Namespace threading deferred** — FNV-1a key generation always uses empty namespace; this will need fixing before multi-file compilation
- **`lower_dlg_text` code duplication** — dialogue text lowering reimplements the same concatenation fold as `lower_fmt_string`; should have called the shared helper

### Patterns Established
- **Owned AST types** — no `'src` lifetime from CST; `String`, `Box<T>`, `Vec<T>` throughout
- **Private fields with method API on LoweringContext** — all access via methods; internal representation can change freely
- **`assert_debug_snapshot` over RON** — avoids needing chumsky serde feature for SimpleSpan
- **INSTA_UPDATE=always for test acceptance** — one-step snapshot workflow
- **Intermediate structs over tuples** — EntityProperty/EntityUseClause/EntityHook provide clear field access
- **Save/restore pattern for scoped state** — speaker stack depth saved before block entry, drained after

### Key Lessons
1. **Audit before completing** — the milestone audit caught real integration bugs that per-phase verification missed (speaker stack leak across sequential dlg items)
2. **Test infrastructure pays for itself immediately** — `lower_src` and `lower_src_with_errors` helpers made adding new tests trivial; 69 tests with minimal boilerplate
3. **Parser defects surface during lowering tests** — entity_property trailing-comma issue (Phase 5 gap closure) was a parser grammar bug caught by lowering snapshot tests
4. **Rust 2024 edition strictness is helpful but requires attention** — explicit direct deps, no inline `use` in match arms, for-loop over flat_map with `&mut` captures

### Cost Observations
- Model mix: predominantly opus (execution), sonnet (research/verification/integration), haiku (plan-check)
- Sessions: ~8 (research + planning + execution across 7 phases + audit + gap closure)
- Notable: 2-day wall-clock for 7 phases / 13 plans / 3,493 LOC — highly efficient for a complete lowering pipeline

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 67 | 7 | Established audit → gap closure cycle; snapshot-first testing |

### Cumulative Quality

| Milestone | Tests | LOC (src) | Tech Debt Items |
|-----------|-------|-----------|-----------------|
| v1.0 | 69 | 3,493 | 7 (all minor) |

### Top Lessons (Verified Across Milestones)

1. Audit before milestone completion catches integration bugs that per-phase verification misses
2. Snapshot tests from the earliest phases compound into comprehensive coverage with minimal late-stage effort
