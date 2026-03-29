# Phase 90: Getting Started and Architecture - Research

**Researched:** 2026-03-27
**Domain:** mdBook prose documentation — installation, hello world, CLI reference, compiler architecture
**Confidence:** HIGH

## Summary

Phase 90 adds five new prose chapters to the existing mdBook site: three under a new "Getting Started"
section (Installation, Hello World, CLI Reference) and two under a new "Architecture" section (Compiler
Pipeline, Crate Map with minimal Contributing notes). The site scaffold, syntax highlighting, and
language-ref wrappers already exist from Phases 87-89. This phase introduces no new build tooling —
it only adds Markdown files and inserts entries into `docs/src/SUMMARY.md`.

The content is grounded entirely in the actual source tree. All facts are verified from the codebase:
the 6-subcommand CLI (new, build, compile, assemble, disasm, run) is fully defined in
`writ-cli/src/main.rs`; the 5-stage pipeline (parse → lower → resolve → typecheck → emit) is named
verbatim in both `writ-cli/src/pipeline.rs` and `writ-compiler/src/lib.rs`; all 10 crate dependency
relationships are extracted from each crate's `Cargo.toml`.

Note that the workspace actually contains **10 crates**, not 9 as stated in the phase requirements.
The README.MD lists all 10: writ-parser, writ-compiler, writ-module, writ-runtime, writ-assembler,
writ-diagnostics, writ-lsp, writ-dap, writ-golden, writ-cli. The ARCH-02 requirement says "9 Rust
crates" — the planner should document all 10 and note the discrepancy. `writ-golden` is a test-only
crate (no binary, no lib exported to end users) so it may be the one excluded from the "9" count in
the requirement; either way, document all 10 for accuracy.

**Primary recommendation:** Write five standalone Markdown files directly in `docs/src/` (no
`{{#include}}` wrappers needed) and add two new top-level sections to `SUMMARY.md` before Language
Reference.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Content Structure and Tone**
- 3 new chapters in a "Getting Started" section before Language Reference: Installation, Hello World, CLI Reference
- 2 new chapters in an "Architecture" section: Compiler Pipeline, Crate Map
- Concise technical writing tone — direct instructions with code examples, minimal narrative, matching the spec's existing style
- Hello World program uses a simple `fn main()` with a `say` dialogue — demonstrates both basic function syntax and Writ's unique dialogue feature in ~5 lines
- Crate dependency diagram presented as an ASCII table showing crate name, purpose, and dependencies — renders cleanly in mdBook without external tools

**CLI Documentation Scope**
- Every subcommand documented with all flags: `writ compile`, `writ run`, `writ build` each with flags, descriptions, and examples
- Architecture page uses prose + table/diagram only, no code snippets (code belongs in cargo doc)
- Minimal "Contributing" section at the end of architecture page pointing to crate map and test commands — not a separate page

### Claude's Discretion
- Exact wording and prose for each page
- CLI flag discovery from actual binary help output
- Crate relationship details from Cargo.toml inspection
- SUMMARY.md placement of new sections

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| START-01 | Installation page covering Rust toolchain prerequisites and building writ from source | README.MD building section confirms `cargo build --workspace` and `cargo test --workspace`; workspace uses edition 2024 (Rust ≥ 1.85 required) |
| START-02 | Hello World walkthrough — creating a .writ file, compiling with writ compile, running with writ run | `writ-cli/src/commands/compile.rs` + `run.rs` confirm invocation pattern; `writ-golden/tests/golden/fn_log_say_choice.writ` provides a working say-based example |
| START-03 | CLI reference page documenting writ compile, writ run, writ build subcommands | `writ-cli/src/main.rs` is authoritative source; all flags confirmed there (see CLI Facts section) |
| ARCH-01 | Compiler pipeline overview (parse → lower → resolve → typecheck → codegen) with crate responsibilities | `writ-cli/src/pipeline.rs` line 1 names stages verbatim; `writ-compiler/src/lib.rs` maps stage → module |
| ARCH-02 | Crate structure diagram showing the 9 Rust crates and their dependencies | All 10 Cargo.toml files read; dependency table in Architecture Patterns section below |
| ARCH-03 | Contribution guide covering build instructions, testing, and PR workflow | Minimal Contributing section at end of architecture page (not separate page per CONTEXT.md) |
</phase_requirements>

---

## Standard Stack

### Core

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| mdBook | 0.4.51 (pinned) | Static site generator | Already configured; 0.5.x breaks mdbook-admonish |
| mdbook-admonish | 1.20.0 | Callout boxes (note/warning/tip) | Already configured in book.toml |

No new tooling is introduced in this phase. All content is pure Markdown rendered by the existing
mdBook configuration.

**Installation command:** None — tooling already installed from Phase 87.

---

## Architecture Patterns

### File Placement

New files go directly in `docs/src/` — no subdirectory, no `{{#include}}` wrapper needed. These are
original prose chapters, not wrappers around spec files.

```
docs/src/
├── SUMMARY.md            (existing — insert new sections)
├── introduction.md       (existing)
├── getting-started/
│   ├── installation.md   (NEW)
│   ├── hello-world.md    (NEW)
│   └── cli-reference.md  (NEW)
├── architecture/
│   ├── compiler-pipeline.md  (NEW)
│   └── crate-map.md          (NEW)
├── language-ref/         (existing)
└── il-spec/              (existing)
```

Using subdirectories (`getting-started/`, `architecture/`) is the clean mdBook convention for
grouped chapters and matches the existing `language-ref/` and `il-spec/` pattern.

### SUMMARY.md Insert Position

New sections go **before** the existing `# Language Reference` section:

```markdown
# Getting Started

- [Installation](getting-started/installation.md)
- [Hello World](getting-started/hello-world.md)
- [CLI Reference](getting-started/cli-reference.md)

# Architecture

- [Compiler Pipeline](architecture/compiler-pipeline.md)
- [Crate Map](architecture/crate-map.md)

# Language Reference
...
```

### Pattern: Standalone Prose Chapter

**What:** A Markdown file written as self-contained prose — no `{{#include}}` directive.
**When to use:** For all 5 new chapters in this phase. Original content that is not sourced from
`language-spec/spec/` files uses direct Markdown, not include wrappers.

### Pattern: admonish Callouts

Admonish is already configured. Use it for tips and warnings that fit the documentation:

```markdown
```admonish tip
Use `writ new <name>` to create a project with the correct directory structure automatically.
```
```

### Anti-Patterns to Avoid

- **H1 in wrapper files:** The `{{#include}}` convention strips H1 from spec files. For new prose
  chapters, the file must start with its own `# Title` H1 since these are not wrapper files.
- **Relative paths anchored to spec source:** Links in spec wrapper files use `../../../language-spec/spec/`
  paths. New prose chapters in `docs/src/getting-started/` use paths like `../language-ref/overview.md`.
- **Hardcoding subcommand flags from memory:** Use `writ-cli/src/main.rs` as the authoritative
  source — the actual `#[arg]` definitions. Do not rely on memory or README.MD for flag details.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Crate dependency graph diagram | Custom SVG/Mermaid | ASCII table in Markdown | mdBook has no Mermaid plugin; ASCII renders in all contexts without plugins; CONTEXT.md decision |
| Callout boxes | Custom HTML | mdbook-admonish | Already configured; produces consistent styled callouts |

---

## CLI Facts (Verified from Source)

All verified from `writ-cli/src/main.rs` (2026-03-27). These are the authoritative flag definitions.

### `writ new <name>`
Creates a new Writ project directory.
- `name` (positional, required): Project name. Alphanumeric, hyphens, underscores only.
- Creates: `<name>/writ.toml`, `<name>/.gitignore`, `<name>/sources/main.writ`, `<name>/bin/configuration/`

### `writ build [path]`
Compiles all `.writ` sources in a project directory (reads `writ.toml`).
- `path` (positional, default: `.`): Project directory containing `writ.toml`
- `--release`: Compile with release profile (strips debug info)
- `--debug`: Compile with debug profile (default; includes debug info)
- `--name <name>`: Override output module name (default: `project.name` from `writ.toml`)
- Output: `build/<profile>/<module_name>.writc`

### `writ compile <input>`
Compiles a single `.writ` source file.
- `input` (positional, required): Input `.writ` file path
- `-o`, `--output <path>`: Output `.writc` path (default: input with `.writ` replaced by `.writc`)
- Always emits debug info (single-file mode)
- Rejects directory input with helpful message pointing to `writ build`

### `writ assemble <input>`
Assembles a `.writil` text file to a binary `.writc` module.
- `input` (positional, required): Input `.writil` file path, or `-` for stdin
- `-o`, `--output <path>`: Output `.writc` path (default: input with `.writil` replaced by `.writc`)

### `writ disasm <input>`
Disassembles a binary `.writc` module to `.writil` text.
- `input` (positional, required): Input `.writc` binary file
- `--verbose`: Include hex byte offsets and opcode comments for each instruction

### `writ run <input>`
Runs a binary `.writc` module.
- `input` (positional, required): Input `.writc` binary file
- `--entry <name>` (default: `main`): Name of exported method to run
- `--interactive`: Enable interactive choice prompts (default: auto-select 0)
- `--verbose`: Print execution stats and GC info after run

**Note on scope:** CONTEXT.md says "every subcommand documented with all flags: writ compile, writ
run, writ build each with flags, descriptions, and examples." The CLI has 6 subcommands total. The
CLI Reference page should document all 6 for completeness, not just the 3 named in requirements,
since the CONTEXT.md locks "every subcommand."

---

## Compiler Pipeline Facts (Verified from Source)

Stage names verified from `writ-cli/src/pipeline.rs` comment (line 1):
`parse -> lower -> resolve -> typecheck -> emit`

Stage-to-crate mapping from `writ-compiler/src/lib.rs`:

| Stage | Stage Number | Function/API | Crate | Input | Output |
|-------|-------------|-------------|-------|-------|--------|
| Parse | 1 | `writ_parser::parse(src)` | writ-parser | `.writ` source string | CST |
| Lower | 2 | `writ_compiler::lower(cst)` | writ-compiler (lower module) | CST | AST (`Ast`) |
| Resolve | 3 | `writ_compiler::resolve::resolve(...)` | writ-compiler (resolve module) | AST | resolved AST |
| Typecheck | 4 | `writ_compiler::check::typecheck(...)` | writ-compiler (check module) | resolved AST | `TypedAst` |
| Emit (codegen) | 5 | `writ_compiler::emit_bodies(...)` | writ-compiler (emit module) | TypedAst | `.writc` bytes |

**Note on requirement wording:** ARCH-01 says "parse, lower, resolve, typecheck, codegen" — the code
calls stage 5 "emit" internally, but "codegen" is acceptable documentation terminology. The plan
should use "codegen" to match the requirement wording, noting it corresponds to the `emit` module.

---

## Crate Dependency Graph (Verified from Cargo.toml)

All 10 crates with their workspace dependencies (external deps omitted):

| Crate | Binary | Purpose | Depends On (workspace crates) |
|-------|--------|---------|-------------------------------|
| writ-diagnostics | — | Diagnostic rendering, error codes, FileId | (none) |
| writ-module | — | IL binary module format (reader/writer, 200-byte header, 21 metadata tables) | (none) |
| writ-parser | — | Lexer (logos) and CST parser (chumsky) | (none) |
| writ-assembler | — | Text IL assembler and disassembler | writ-module |
| writ-compiler | — | Name resolution, type checking, IL code generation | writ-parser, writ-diagnostics, writ-module |
| writ-runtime | — | Register-based VM, task scheduler, entity system, GC | writ-module (writ-compiler optional feature) |
| writ-lsp | writ-lsp | Language server (LSP) for editor intelligence | writ-compiler, writ-diagnostics, writ-parser, writ-module, writ-runtime |
| writ-dap | writ-dap | Debug adapter (DAP) for source-level debugging | writ-compiler, writ-runtime, writ-module, writ-diagnostics, writ-parser |
| writ-golden | — | Golden file integration tests | writ-compiler, writ-assembler, writ-module, writ-diagnostics, writ-parser |
| writ-cli | writ | `writ` command-line tool | writ-module, writ-runtime, writ-assembler, writ-compiler, writ-diagnostics, writ-parser |

**Foundation crates (no workspace deps):** writ-diagnostics, writ-module, writ-parser

**Clarification on "9 crates":** ARCH-02 requirement says 9 crates; the workspace has 10. `writ-golden`
is a dev/test-only crate (no exported binary, test-only entry point). The crate map should document
all 10 with a note that writ-golden is test infrastructure only.

---

## Hello World Program

The CONTEXT.md locks: "uses a simple `fn main()` with a `say` dialogue — demonstrates both basic
function syntax and Writ's unique dialogue feature in ~5 lines."

`say` requires a speaker `Entity` argument (confirmed from `cli_host.rs`: `say(speaker, text)` is
spec §1.27.4). A minimal say-based hello world requires an entity. The golden test
`fn_log_say_choice.writ` demonstrates the pattern:

```writ
entity Narrator {}

pub fn main() {
    let speaker: Entity = Entity.getOrCreate<Narrator>();
    ::say(speaker, "Hello, World!");
}
```

This is ~5 lines as required, shows `fn main()`, shows the `say` dialogue builtin, and requires
`Entity.getOrCreate<Narrator>()` which demonstrates entity usage. The `::` prefix calls a builtin.

**CLI invocation sequence for walkthrough:**
```
writ compile hello.writ
writ run hello.writc
```

**Expected output from writ run:**
```
[say] <entity@0>: Hello, World!
```

The CliHost prints `[say] {speaker}: {text}` format. The speaker is printed as the entity value
because `Entity` resolves to an entity handle, not a string name.

---

## Installation Prerequisites (Verified)

From workspace `Cargo.toml`: `resolver = "3"` — requires Rust edition 2024 workspace resolver.
From crate Cargo.toml files: all use `edition = "2024"`.

Rust edition 2024 requires **Rust 1.85 or newer** (stable channel). The installation page must
specify this minimum version.

**Build commands (from README.MD):**
```bash
cargo build --workspace        # debug build
cargo build --workspace --release  # release build (enables LTO + codegen-units=1)
cargo test --workspace         # run all tests
```

**Release profile (from workspace Cargo.toml):**
```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
```

The release build is significantly slower to compile due to fat LTO. The installation page should
mention this.

---

## Common Pitfalls

### Pitfall 1: SUMMARY.md Section Placement
**What goes wrong:** New sections added after Language Reference instead of before it.
**Why it happens:** Appending to the end of SUMMARY.md is the path of least resistance.
**How to avoid:** CONTEXT.md explicitly requires Getting Started before Language Reference. Insert
the two new sections immediately before the `# Language Reference` line.
**Warning signs:** Site navigation shows Getting Started below IL Specification.

### Pitfall 2: H1 Title in New Chapter Files
**What goes wrong:** Omitting the `# Title` H1 from a new prose chapter, causing a blank sidebar
title or the first sentence rendered as the title.
**Why it happens:** The existing wrapper files (language-ref/*.md) intentionally omit H1 because
the `{{#include}}` pulls it from the spec file. New prose chapters must include their own H1.
**How to avoid:** Every new chapter file starts with `# Chapter Title`.
**Warning signs:** Chapter sidebar entry appears blank in `mdbook serve`.

### Pitfall 3: Incorrect say() Signature in Hello World
**What goes wrong:** Writing `::say("Hello, World!")` without a speaker argument.
**Why it happens:** Intuitive "print" analogy, but `say` takes `(speaker: Entity, text: string)`.
**How to avoid:** Use `Entity.getOrCreate<Narrator>()` to obtain a speaker first.
**Warning signs:** Compiler error E0002 (wrong argument count) or type error.

### Pitfall 4: writ run on .writ File Instead of .writc
**What goes wrong:** `writ run hello.writ` fails with module parse error.
**Why it happens:** `writ run` takes a compiled `.writc` binary, not source.
**How to avoid:** Hello World walkthrough must clearly show the two-step flow: compile first, then run.
**Warning signs:** "failed to parse module" error from writ run.

### Pitfall 5: Documenting CLI Scope Incompletely
**What goes wrong:** CLI Reference page covers only compile/run/build but omits new/assemble/disasm.
**Why it happens:** ARCH-01 requirement mentions only three subcommands; CONTEXT.md says "every subcommand."
**How to avoid:** Document all 6 subcommands. The CONTEXT.md constraint ("every subcommand
documented") overrides the shorter list in the requirement description.
**Warning signs:** User asks about `writ new` and finds no documentation.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Docs in README.MD only | mdBook site at /Writ/ | Phase 87 (v9.0) | Full browsable documentation site |
| No syntax highlighting for Writ code | Custom highlight.js in docs/theme/ | Phase 88 (v9.0) | Code blocks in new chapters get Writ highlighting automatically |
| No cross-references | mdBook internal links via Phase 89 | Phase 89 (v9.0) | New chapters can link to language-ref chapters with working navigation |

---

## Open Questions

1. **Hello World speaker output format**
   - What we know: CliHost prints `[say] <entity@0>: Hello, World!` — the speaker renders as `<entity@0>` not as "Narrator"
   - What's unclear: Should the walkthrough explain this format as intentional CLI behavior, or is there a way to produce a cleaner name?
   - Recommendation: Document the actual output `[say] <entity@0>: Hello, World!` verbatim and add a note that `[say]` is the CLI's annotation format; real game hosts provide their own display.

2. **Rust minimum version**
   - What we know: `edition = "2024"` requires Rust 1.85+; workspace uses resolver 3
   - What's unclear: Whether there is an `rust-version` field set anywhere that makes this explicit
   - Recommendation: Check if any crate has `rust-version` key; if not, state "Rust 1.85 or later" based on edition 2024 requirement.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies beyond existing mdBook toolchain, which was verified in Phase 87).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (existing) |
| Config file | No separate test config — all Cargo-native |
| Quick run command | `cargo test -p writ-golden` |
| Full suite command | `cargo test --workspace` |

This phase adds only Markdown files and SUMMARY.md edits. There are no new Rust tests.
Validation is structural: `mdbook build` must succeed, and the 5 new chapters must appear in
the navigation. No automated test framework covers mdBook page existence, but a build smoke test
confirms chapter wiring.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| START-01 | Installation page renders in mdBook | smoke | `mdbook build docs/ 2>&1 | grep -c error` | ❌ Wave 0 |
| START-02 | Hello World page renders with code block | smoke | `mdbook build docs/ 2>&1 | grep -c error` | ❌ Wave 0 |
| START-03 | CLI Reference page renders, all 6 subcommands present | manual | Open built site, verify subcommand sections | N/A |
| ARCH-01 | Compiler Pipeline page renders | smoke | `mdbook build docs/ 2>&1 | grep -c error` | ❌ Wave 0 |
| ARCH-02 | Crate Map page renders with ASCII table | smoke | `mdbook build docs/ 2>&1 | grep -c error` | ❌ Wave 0 |
| ARCH-03 | Contributing section at end of crate map page | manual | Open built site, verify section exists | N/A |

### Sampling Rate
- **Per task commit:** `cd docs && mdbook build 2>&1 | tail -3` (verify no errors)
- **Per wave merge:** Full mdbook build from clean state
- **Phase gate:** `mdbook build` exits 0 with 5 new chapters navigable before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `docs/src/getting-started/` directory — must be created
- [ ] `docs/src/architecture/` directory — must be created
- Five new `.md` files as listed in Architecture Patterns section

*(No new Rust test files required — this is documentation-only work)*

---

## Sources

### Primary (HIGH confidence)
- `writ-cli/src/main.rs` — authoritative CLI subcommand and flag definitions (read directly)
- `writ-cli/src/pipeline.rs` — authoritative 5-stage pipeline stage names (read directly)
- `writ-compiler/src/lib.rs` — stage-to-module mapping (read directly)
- All 10 `*/Cargo.toml` files — crate dependency graph (read directly)
- `docs/src/SUMMARY.md` — existing navigation structure (read directly)
- `docs/book.toml` — existing configuration (read directly)
- `docs/src/language-ref/overview.md` — existing wrapper file pattern (read directly)

### Secondary (MEDIUM confidence)
- `README.MD` — build commands and project structure overview (read directly)
- `writ-golden/tests/golden/*.writ` — real working Writ programs for Hello World example (read directly)

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new tooling; existing mdBook configuration unchanged
- Architecture: HIGH — all facts from source files, zero inference
- Pitfalls: HIGH — derived from code inspection (say signature, run expects .writc, etc.)
- CLI facts: HIGH — read directly from `writ-cli/src/main.rs` `#[arg]` annotations
- Pipeline facts: HIGH — read from `pipeline.rs` and `lib.rs` docstrings

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (stable codebase; CLI changes would invalidate CLI section)
