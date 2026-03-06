# Phase 40: Spec Cleanup - Research

**Researched:** 2026-03-06
**Domain:** Language spec markdown editing, Rust serde configuration, writ.toml field alignment
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SPEC-01 | Spec §1.2.8 (Serialization Critical Sections) is removed — section deleted, surrounding numbering updated | File `37_2_8_serialization_critical_sections_removed.md` and TOC entry identified; no code changes needed |
| SPEC-02 | Spec clarifies that `log`, `say`, and `choice` are root-namespace inbuilt calls — no qualification needed | Existing spec says "Runtime namespace" and "not callable directly"; golden test uses `::log`/`::say`/`::choice` as root prefix; new spec section needed |
| SPEC-03 | `writ.toml` field names aligned: `locale.default`, `locale.supported`, consistent sources directory | Config.rs uses `default_locale`/`locales`; scaffold uses `default`/`supported`; sources mismatch identified |
</phase_requirements>

---

## Summary

Phase 40 is a documentation-only cleanup phase. All three requirements are markdown edits and one Rust struct update — no compiler logic, no new algorithms, no runtime changes.

**SPEC-01** is the simplest: one markdown file (`37_2_8_serialization_critical_sections_removed.md`) must be deleted and the corresponding TOC entry removed. The §3.16 REMOVED section (instruction-set side of the same change) can remain as it documents an opcode gap.

**SPEC-02** requires adding a new spec subsection to §26 (Standard Library Builtins) or §23 (Modules & Namespaces) that explicitly states `log`, `say`, and `choice` are root-namespace inbuilt functions, callable without any qualifier. The current spec says they live in the "Runtime namespace" (§13.9, §28.5) and are "not callable directly" — this contradicts the way the compiler lowers dialogue and the way every code example in the spec uses `log()` without a prefix. The fix is new spec prose; no code changes.

**SPEC-03** is the only change with a code component. The `LocaleConfig` struct in `config.rs` uses Rust field names `default_locale` and `locales` which do not match the TOML keys `default` and `supported` that the spec documents and that `writ new` generates. The scaffold also has an inconsistency: it creates a `sources/` directory but the TOML has `sources` commented out, causing `load_config` to default to `src/`. The fix is two `#[serde(rename)]` attributes on `LocaleConfig` fields and uncommenting `sources = ["sources/"]` in the scaffold template.

**Primary recommendation:** Three focused tasks — delete §2.8 file + TOC entry, write a "Root-namespace inbuilts" spec subsection, and fix the two serde renames + scaffold sources line.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.x | TOML deserialization | Already a dependency in writ-compiler |
| toml | 0.9 | TOML parsing | Already a dependency in writ-compiler |

No new dependencies required for this phase. The `#[serde(rename = "...")]` attribute is standard serde and requires no extra crates.

**Installation:** Nothing to install — all dependencies already present.

---

## Architecture Patterns

### Pattern 1: Serde Field Renaming
**What:** Use `#[serde(rename = "toml_key")]` to decouple Rust field names from serialized TOML keys.
**When to use:** When the TOML key name is a Rust reserved word or conflicts with naming conventions.
**Example:**
```rust
// Source: serde documentation
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleConfig {
    #[serde(rename = "default")]
    pub default_locale: String,
    #[serde(rename = "supported")]
    #[serde(default)]
    pub locales: Vec<String>,
}
```
Note: `default` is a reserved keyword in Rust, so renaming the field while using `serde(rename)` is the correct approach. The Rust field stays `default_locale`; only the TOML key changes to `default`.

### Pattern 2: Spec Section Deletion
**What:** Remove a markdown file and its TOC entry from the splatted spec.
**When to use:** When a design section is fully superseded and keeping a REMOVED stub causes confusion.
**Steps:**
1. Delete `language-spec/spec/37_2_8_serialization_critical_sections_removed.md`.
2. Remove the corresponding line from `language-spec/spec/01_table_of_contents.md`.
3. File numbering gaps (37_ missing) are acceptable — splatted files are ordered by prefix, not required to be contiguous.

### Pattern 3: Adding a Spec Subsection
**What:** Insert a new subsection into an existing spec file.
**When to use:** When the spec uses something without defining it.
**Placement:** The new "root-namespace inbuilt calls" content belongs in `27_26_standard_library_builtins.md` as a new §26.4. This is the natural home because §26 already documents compiler-known types and contracts.

### Anti-Patterns to Avoid
- **Renaming Rust fields to `default`**: `default` is a Rust reserved keyword — it cannot be used as a field name without `r#default` syntax. Use `serde(rename)` instead.
- **Changing the TOML key `sources`**: The `CompilerConfig.sources` field already has the right Rust name and the spec already uses `sources`. Only the `LocaleConfig` fields need fixing.
- **Deleting §3.16 as part of this phase**: §3.16 (`64_3_16_serialization_control_removed.md`) documents an opcode assignment gap in the instruction table — different from §2.8. SPEC-01 only covers §2.8.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML key aliasing | Manual string matching in `load_config` | `#[serde(rename)]` | One attribute, zero runtime cost, standard pattern |
| Test for serde rename | New test file | Inline test in `config.rs` `#[cfg(test)]` block | Tests already live there; follow existing pattern |

---

## Common Pitfalls

### Pitfall 1: The `default` Rust Keyword
**What goes wrong:** Renaming the Rust field to `default` causes a compile error — `default` is a reserved keyword.
**Why it happens:** TOML uses `default` as a plain key but Rust reserves it for specialization (even though specialization is nightly-only, the keyword is still reserved).
**How to avoid:** Keep the Rust field as `default_locale` and add `#[serde(rename = "default")]`.
**Warning signs:** `error[E0578]: expected identifier, found keyword 'default'`

### Pitfall 2: Forgetting `#[serde(default)]` on `locales`
**What goes wrong:** A `writ.toml` without `supported = [...]` fails to deserialize with a "missing field" error.
**Why it happens:** `Vec<String>` has no implicit default in serde — unlike `Option<T>`.
**How to avoid:** Add `#[serde(default)]` alongside the `#[serde(rename = "supported")]` annotation. The default is an empty `Vec`, which means "only the default locale is targeted."
**Warning signs:** Test with minimal `writ.toml` (no locale section at all) panics or returns a parse error.

### Pitfall 3: Scaffold `sources` Directory vs Default
**What goes wrong:** `writ new` creates `sources/main.writ` but `config.rs` defaults to `["src/"]`. The scaffold's writ.toml has `sources` commented out, so `load_config` never finds the `sources/` directory.
**Why it happens:** The scaffold was written with a different directory convention than the config default.
**How to avoid:** Uncomment `sources = ["sources/"]` in the scaffold template (the `toml_content` string in `cmd_new` in `writ-cli/src/main.rs`).
**Warning signs:** `discover_source_files` returns 0 files on a freshly-scaffolded project.

### Pitfall 4: Forgetting to Update Existing Tests in `config.rs`
**What goes wrong:** After adding `#[serde(rename)]`, the inline `parse_basic_config` test still uses `default_locale = "en"` and `locales = [...]` as TOML keys — these tests will now fail because the keys are renamed.
**Why it happens:** Tests were written against the old (wrong) key names.
**How to avoid:** Update the test TOML strings to use `default = "en"` and `supported = ["en", "ja"]`.

### Pitfall 5: TOC Anchor Mismatch After Deletion
**What goes wrong:** Removing the §2.8 line from the TOC leaves a dangling anchor reference if anything cross-links to it.
**Why it happens:** The file `37_2_8_serialization_critical_sections_removed.md` is referenced only from the TOC — no other file links to it by anchor (confirmed by grep: only `01_table_of_contents.md` contains the anchor).
**How to avoid:** After deletion, grep for `2_8` and `§2.8` to confirm no dangling references remain.

### Pitfall 6: §26 vs §28.5 Conflict After SPEC-02 Addition
**What goes wrong:** §13.9 says `say`/`choice` are "not callable directly from user code under the Runtime prefix" but §28.5 says "runtime must provide these core functions in the Runtime namespace." After adding a clarifying section, these three places may say contradictory things.
**Why it happens:** The spec was written at different times with inconsistent framing.
**How to avoid:** Update §13.9 and §28.5 to reflect the new framing: they are root-namespace inbuilts, accessible as `log()`, `say()`, `choice()` without any prefix. The compiler lowers dialogue syntax to these calls. The old "Runtime namespace" wording should be removed from §13.9 and changed to a footnote in §28.5.

---

## Code Examples

### Correct serde rename pattern
```rust
// Source: serde docs, standard pattern
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleConfig {
    /// Default locale identifier (TOML key: `default`).
    #[serde(rename = "default")]
    pub default_locale: String,
    /// Supported locale identifiers (TOML key: `supported`).
    #[serde(rename = "supported")]
    #[serde(default)]
    pub locales: Vec<String>,
}
```

### Updated inline test TOML (after rename fix)
```toml
[project]
name = "test-game"
version = "0.1.0"

[locale]
default = "en"
supported = ["en", "ja"]

[compiler]
sources = ["src/", "scripts/"]
output = "build/"
```

### Updated scaffold sources line in `cmd_new`
The scaffold template in `writ-cli/src/main.rs` currently has:
```toml
# sources = ["sources/", "dialogue/"]
```
Change to uncomment and match the scaffold-created directory:
```toml
sources = ["sources/"]
```

### Spec prose for §26.4 (new section for SPEC-02)
```markdown
### 26.4 Root-Namespace Inbuilt Calls

Three functions are always available in the root namespace without any qualifier:

| Function | Signature | Purpose |
|----------|-----------|---------|
| `log`    | `fn log(msg: string)` | Write a message to the host's debug log |
| `say`    | `fn say(speaker: Entity, text: string)` | Display dialogue (transition point — suspends) |
| `choice` | `fn choice(options: ...) -> int` | Present choices (transition point — suspends) |

These are **inbuilt calls** — the compiler resolves them from the root namespace. They are callable as
`log(msg)`, `say(speaker, text)`, and `choice(options)`. No `writ::`, `Runtime::`, or any other
qualifier is needed or accepted.

`say` and `choice` are dialogue transition points — the VM suspends until the host responds (§13.9).
`log` is a fire-and-forget debug output call; it does not suspend.

The compiler lowers `dlg` syntax (`@Speaker text`, `$ choice { ... }`) into calls to `say` and
`choice` automatically — user code in `dlg` blocks does not call them directly. In `fn` bodies,
`log` may be called freely.
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CRITICAL_BEGIN/CRITICAL_END instructions | Suspend-and-confirm model (§2.14.2) | Resolved in earlier milestone | §2.8 no longer needed; file should be deleted |
| `LocaleConfig.default_locale` / `.locales` TOML keys | `default` / `supported` TOML keys | This phase | Deserialization of scaffolded projects currently broken |
| `sources/` default (scaffold creates it) | `src/` (config default) | This phase | discover_source_files finds 0 files on new projects |

**Deprecated/outdated:**
- §2.8 Serialization Critical Sections REMOVED stub: should be deleted entirely, not kept as a REMOVED marker.
- "Runtime namespace" framing for `say`/`choice` in §13.9 and §28.5: misleading — these are root-namespace inbuilts.
- `default_locale`/`locales` TOML key names in config.rs tests: update to `default`/`supported`.

---

## Open Questions

1. **Should §3.16 (Serialization Control REMOVED) also be deleted?**
   - What we know: §3.16 (`64_3_16_serialization_control_removed.md`) is a different section from §2.8 — it documents the removal of `CRITICAL_BEGIN`/`CRITICAL_END` instructions from the opcode table.
   - What's unclear: SPEC-01 says "§1.2.8 is removed". Whether this covers §3.16 as well is ambiguous.
   - Recommendation: Keep §3.16 for now. It serves a different purpose (explaining an opcode table gap). Include a note in the plan about this scoping decision.

2. **Where exactly should the inbuilt calls section live?**
   - What we know: §26 (Standard Library Builtins) already has §26.1 (types), §26.2 (contracts), §26.3 (std types).
   - What's unclear: Whether §26 is the right home or if a dedicated "Runtime-Provided Functions" section fits better.
   - Recommendation: Add as §26.4 — it logically extends "what's always available without importing."

3. **Should the existing `locales` field be renamed in Rust too?**
   - What we know: The requirement says field names should "exactly match" with `#[serde(rename)]` if needed.
   - What's unclear: Whether "exactly match" means the TOML key should drive the Rust field name, or just that round-trip works.
   - Recommendation: Keep Rust field as `default_locale` and `locales` (they're readable and avoid the `default` keyword issue), using only `serde(rename)` to fix the TOML key. This is the cleanest solution.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (built-in) + `insta` for snapshot tests |
| Config file | `Cargo.toml` per crate |
| Quick run command | `cargo test -p writ-compiler config` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPEC-01 | §2.8 file deleted from spec | manual | manual inspection | N/A |
| SPEC-02 | §26.4 section exists with correct prose | manual | manual inspection | N/A |
| SPEC-03 | `load_config` deserializes scaffold-generated writ.toml without error | unit | `cargo test -p writ-compiler config` | Partial (tests exist, need updating) |
| SPEC-03 | `load_config` correctly parses `default`/`supported` keys | unit | `cargo test -p writ-compiler config::tests::parse_basic_config` | Partial (test exists, TOML string needs updating) |
| SPEC-03 | `load_config` handles absent `supported` field gracefully | unit | `cargo test -p writ-compiler config::tests::default_sources_when_omitted` | Partial (analog needed for locale) |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler config`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Test `config::tests::parse_basic_config` — update TOML string from `default_locale`/`locales` to `default`/`supported`
- [ ] Test `config::tests::locale_without_supported` — new test confirming absent `supported` field deserializes to empty `Vec`
- [ ] Test `config::tests::scaffold_toml_round_trips` — parse the exact TOML generated by `cmd_new` through `load_config` and assert no error

---

## Detailed Findings by Requirement

### SPEC-01: Delete §2.8 Serialization Critical Sections

**File to delete:** `language-spec/spec/37_2_8_serialization_critical_sections_removed.md`

**TOC entry to remove (in `01_table_of_contents.md`):**
```
  * [2.8 Serialization Critical Sections — REMOVED](#28-serialization-critical-sections--removed)
```

**Cross-references verified:** grep for `§2.8`, `2_8`, `CRITICAL_BEGIN`, `CRITICAL_END` in all spec files. No other file links to §2.8 by anchor. The `64_3_16_serialization_control_removed.md` references "§2.14.2" not §2.8 — no dependency.

**File numbering after deletion:** Files are named `00_` through `69_` with one gap at `37_` after deletion. This is acceptable — files are consumed alphabetically and gaps are fine.

**Confidence:** HIGH

---

### SPEC-02: Root-Namespace Inbuilt Calls

**Current state (problematic):**

§13.9 (`14_13_dialogue_blocks_dlg.md`) says:
> "The core dialogue functions live in the `Runtime` namespace... These functions are not callable directly from user code under the `Runtime` prefix."

§28.5 (`29_28_lowering_reference.md`) says:
> "The runtime must provide these core functions in the `Runtime` namespace."

The golden test `fn_log_say_choice.writ` uses `::log`, `::say`, `::choice` — `::` prefix means root namespace, not `Runtime::`. This is correct behavior.

Every code example in the spec uses `log(msg)` without any qualifier. The spec's framing in §13.9 is contradictory and misleading.

**Fix scope:**
1. Add §26.4 to `27_26_standard_library_builtins.md` — defines the three root-namespace inbuilts.
2. Update §13.9 in `14_13_dialogue_blocks_dlg.md` — remove "Runtime namespace" wording; replace with "root-namespace inbuilt calls (§26.4)".
3. Update §28.5 in `29_28_lowering_reference.md` — remove "Runtime namespace" from the table preamble; add a note that these are root-namespace inbuilts.

**No code changes required** — the compiler already resolves `say`/`choice`/`log` as plain unqualified names. The fix is spec-only.

**Confidence:** HIGH

---

### SPEC-03: writ.toml Field Alignment

**Mismatch table:**

| What | Spec TOML Key | Config.rs Rust Field | Config.rs Deserialization Key |
|------|--------------|---------------------|-------------------------------|
| Default locale | `locale.default` | `default_locale` | `default_locale` (WRONG) |
| Supported locales | `locale.supported` | `locales` | `locales` (WRONG) |
| Sources directory | `compiler.sources` | `sources` | `sources` (CORRECT) |

**Scaffold state (writ-cli/src/main.rs `cmd_new`):**

| What | Scaffold generates | Config.rs deserializes |
|------|--------------------|------------------------|
| Locale default key | `default = "en"` | Expects `default_locale` → FAILS |
| Locale supported key | `# supported = [...]` (commented out) | Would need `supported` key after fix |
| Sources directory (physical) | Creates `sources/` | Defaults to `["src/"]` → MISMATCH |
| Sources TOML key | `# sources = ["sources/", ...]` (commented out) | Uses default `["src/"]` |

**Fix in `config.rs`:**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleConfig {
    #[serde(rename = "default")]
    pub default_locale: String,
    #[serde(rename = "supported")]
    #[serde(default)]
    pub locales: Vec<String>,
}
```

**Fix in `writ-cli/src/main.rs` scaffold template:**
Change the commented `sources` line to:
```toml
sources = ["sources/"]
```
(This ensures `discover_source_files` finds the `sources/main.writ` created by the scaffold.)

**Fix inline tests in `config.rs`:** Update the test TOML from `default_locale = "en"` / `locales = [...]` to `default = "en"` / `supported = [...]`.

**Confidence:** HIGH

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection — `writ-compiler/src/config.rs` — field names and serde attributes
- Direct code inspection — `writ-cli/src/main.rs` `cmd_new` — scaffold template
- Direct spec reading — `language-spec/spec/03_2_project_configuration_writ_toml.md` — authoritative TOML keys
- Direct spec reading — `language-spec/spec/37_2_8_serialization_critical_sections_removed.md` — file to delete
- Direct spec reading — `language-spec/spec/01_table_of_contents.md` — TOC entry to remove
- Direct spec reading — `language-spec/spec/27_26_standard_library_builtins.md` — insertion point for §26.4
- Direct spec reading — `language-spec/spec/14_13_dialogue_blocks_dlg.md` §13.9 — "Runtime namespace" wording to fix
- Direct spec reading — `language-spec/spec/29_28_lowering_reference.md` §28.5 — "Runtime namespace" wording to fix
- Direct file inspection — `writ-golden/tests/golden/fn_log_say_choice.writ` — confirms `::log`/`::say`/`::choice` as root-namespace calls

### Secondary (MEDIUM confidence)
- serde documentation pattern: `#[serde(rename)]` is a standard attribute, behavior well-known

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, uses existing serde
- Architecture: HIGH — all changes are mechanical (rename attributes, prose additions, file deletion)
- Pitfalls: HIGH — all pitfalls identified from direct code inspection

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain — spec text and config.rs don't change without deliberate action)
