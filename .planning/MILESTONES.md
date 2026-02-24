# Milestones

## Active
(none — next milestone pending)

## Completed
- **v1.0** — CST-to-AST Lowering Pipeline (shipped 2026-02-27)

---

## v1.0 CST-to-AST Lowering Pipeline (Shipped: 2026-02-27)

**Delivered:** Complete multi-pass CST-to-AST lowering pipeline for the Writ compiler, desugaring all higher-level constructs into spec-defined primitives.

**Phases:** 7 | **Plans:** 13 | **Tests:** 69 snapshot tests
**LOC:** 3,493 Rust (src) | **Commits:** 67
**Timeline:** 2 days (2026-02-26 → 2026-02-27)
**Git range:** `99d6ad6..26837c2`

**Key accomplishments:**
1. AST type hierarchy and pipeline infrastructure — AstExpr/AstStmt/AstDecl/AstType enums with span preservation, LoweringContext with error accumulation, public `lower()` API
2. Expression-level desugaring — Optional sugar (`T?` → `Option<T>`), formattable strings (interpolation → concatenation), compound assignments (`+=`/`-=`/etc.)
3. Operator and concurrency lowering — Operator overloads desugar to contract impls with auto-generated derived operators; concurrency primitives pass through as first-class AST nodes
4. Dialogue lowering with localization — `dlg` → `fn` transformation with three-tier speaker resolution, FNV-1a localization keys, `$ choice` branch scoping, `->` transition validation
5. Entity lowering — `entity` → struct + `ComponentAccess<T>` impls + lifecycle hook registrations with `[Singleton]` propagation and component field flattening
6. Pipeline integration and quality gate — End-to-end pipeline with 69 insta snapshot tests, localization key determinism, speaker stack isolation fix, dead code cleanup

**Tech debt carried forward:**
- R3: `?` propagation and `!` unwrap deferred (spec §18 features)
- R4: Escaped braces (`{{`/`}}`) — lexer gap in writ-parser, not lowering
- R9: Namespace not threaded to FNV-1a key generation
- R4/R8: `lower_dlg_text` duplicates `lower_fmt_string` fold logic

**Archives:** `milestones/v1.0-ROADMAP.md`, `milestones/v1.0-REQUIREMENTS.md`, `milestones/v1.0-MILESTONE-AUDIT.md`

---

