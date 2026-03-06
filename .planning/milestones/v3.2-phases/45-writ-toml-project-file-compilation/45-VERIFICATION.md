---
phase: 45-writ-toml-project-file-compilation
verified: 2026-03-06T22:30:00Z
status: passed
score: 7/7 must-haves verified
re_verification: false
---

# Phase 45: writ.toml Project File Compilation Verification Report

**Phase Goal:** Running `writ compile .` or `writ build` in a directory containing `writ.toml` compiles all .writ source files into one module; `--release` and `--debug` flags are respected
**Verified:** 2026-03-06
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | `ProfileConfig`/`ProfilesConfig` structs deserialize from `[profile.debug]` and `[profile.release]` TOML sections | VERIFIED | `config.rs` lines 50-87: both structs with serde defaults exist. 3 unit tests confirm deserialization behavior (`profile_defaults_when_omitted`, `profile_explicit_override`, `profile_partial_override`). |
| 2  | `WritConfig` gains a `profile` field with sensible defaults (debug_info=true for debug, false for release) | VERIFIED | `config.rs` line 24-26: `#[serde(default)] pub profile: ProfilesConfig`. Default impl at lines 80-87 sets `debug: true`, `release: false`. |
| 3  | `emit_bodies` accepts `emit_debug_info: bool` that gates DebugLocal row emission | VERIFIED | `emit/mod.rs` line 68-73: signature confirmed. `serialize.rs` lines 266-275: `if emit_debug_info { build_debug_locals(...) } else { Vec::new() }`. Header flags gated at line 350. |
| 4  | Existing tests and callers pass `true` for `emit_debug_info` (no behavior change) | VERIFIED | All 7 call sites confirmed: `cmd_compile` passes `true` (main.rs line 509), golden tests pass `true` (line 78), e2e tests pass `true` (line 61), `emit_body_tests` passes `true` (line 2301), `emit_serialize_tests` passes `true` at 4 call sites. |
| 5  | `writ build` discovers all .writ files and compiles them into `{output_base}/{profile}/{name}.writc` | VERIFIED | `cmd_build` at main.rs lines 401-475: calls `load_config`, `discover_source_files`, spawns 16MB-stack thread with `run_pipeline`, writes to `project_root/output_base/profile_name/module_name.writc` with `create_dir_all`. |
| 6  | `--release` uses `config.profile.release.debug_info`; `--debug` (default) uses `config.profile.debug.debug_info` | VERIFIED | main.rs lines 415-417: `let profile_cfg = if release { &config.profile.release } else { &config.profile.debug }; let emit_debug_info = profile_cfg.debug_info;`. Passed to `run_pipeline` at line 457. |
| 7  | Spec section 2.7 documents `[profile.debug]` and `[profile.release]` TOML sections | VERIFIED | `language-spec/spec/03_2_project_configuration_writ_toml.md` lines 118-141: section 2.7 Build Profiles present with table, description, output path pattern, and example. Profile sections also appear in section 2.3 Optional Fields example (lines 40-46). |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/config.rs` | ProfileConfig, ProfilesConfig structs with serde deserialization | VERIFIED | Lines 50-87: both structs present with correct serde defaults. `WritConfig.profile` field at line 24-26. 3 new unit tests at lines 267-312. |
| `writ-compiler/src/emit/mod.rs` | `emit_bodies` with `emit_debug_info: bool` parameter | VERIFIED | Lines 68-73: signature matches; passes flag to `serialize::serialize` at line 137. |
| `writ-compiler/src/emit/serialize.rs` | `translate()` gates DebugLocal emission on `emit_debug_info` | VERIFIED | Lines 25-30 (translate signature), 266-275 (gated DebugLocal build), 350 (gated header.flags), 356-363 (serialize signature passing flag through). |
| `writ-cli/src/main.rs` | Build subcommand, `cmd_build`, `run_pipeline` helper | VERIFIED | Lines 41-57: `Build` variant in `Commands` enum. Lines 321-397: `run_pipeline` helper. Lines 401-475: `cmd_build`. Lines 479-530: refactored `cmd_compile` using helper. |
| `language-spec/spec/03_2_project_configuration_writ_toml.md` | Spec amendment with `[profile.debug]` and section 2.7 | VERIFIED | Lines 40-46: profile sections in 2.3 example. Lines 118-141: full section 2.7 Build Profiles. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `emit/mod.rs` | `emit/serialize.rs` | `emit_debug_info` passed to `serialize::serialize` | WIRED | `mod.rs` line 137: `serialize::serialize(&mut builder, &bodies, interner, emit_debug_info)`. `serialize.rs` line 29 accepts it; passes to `translate` at line 362. |
| `writ-cli/src/main.rs cmd_compile` | `emit/mod.rs emit_bodies` | `run_pipeline` calls `emit_bodies` with `true` | WIRED | `run_pipeline` line 390: `writ_compiler::emit_bodies(..., emit_debug_info)`. `cmd_compile` invokes `run_pipeline` with `true` (line 509). |
| `cmd_build` | `config.rs load_config + discover_source_files` | `cmd_build` calls both | WIRED | main.rs lines 405: `load_config`, line 423: `discover_source_files`. |
| `cmd_build` | `emit/mod.rs emit_bodies` | `run_pipeline` called with `emit_debug_info` from profile | WIRED | main.rs line 457: `run_pipeline(file_sources, None, emit_debug_info)` where `emit_debug_info` was derived from the profile at line 417. |
| `cmd_build` | `config.rs ProfileConfig` | Reads `config.profile.debug` or `config.profile.release` | WIRED | main.rs lines 416-417: `let profile_cfg = if release { &config.profile.release } else { &config.profile.debug }; let emit_debug_info = profile_cfg.debug_info;` |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TOOL-01 | 45-02-PLAN.md | User can compile a Writ project by running `writ compile .` or `writ build` in a directory with `writ.toml` — all `.writ` files compiled into one module | SATISFIED | `writ build` subcommand: `cmd_build` loads `writ.toml`, calls `discover_source_files`, compiles all .writ files via `run_pipeline`, outputs single `.writc`. `writ compile .` on directory returns helpful error pointing to `writ build` (main.rs lines 481-485). |
| TOOL-02 | 45-01-PLAN.md, 45-02-PLAN.md | `--release` and `--debug` flags respected; `[profile.release]` and `[profile.debug]` in `writ.toml` | SATISFIED | Plan 01 adds `ProfileConfig`/`ProfilesConfig` types and `emit_debug_info` pipeline gating. Plan 02 wires `--release`/`--debug` CLI flags to profile selection in `cmd_build`. |

Both TOOL-01 and TOOL-02 are marked Complete in REQUIREMENTS.md traceability table (Phase 45).

### Anti-Patterns Found

No blocker anti-patterns detected. The `_module_name` parameter in `run_pipeline` is correctly prefixed with `_` to indicate intentional non-use (reserved for future use), documented in the function doc-comment. This is a design decision, not a stub.

### Human Verification Required

The following items require manual execution to confirm end-to-end behavior:

#### 1. writ build debug mode produces larger output than release mode

**Test:** Run `writ new test45 && cd test45 && writ build` then `writ build --release` in a temp directory. Compare byte sizes of `build/debug/test45.writc` vs `build/release/test45.writc`.
**Expected:** Debug module is larger (includes DebugLocal entries); release module is smaller (DebugLocal stripped). SUMMARY reports 492 bytes vs 486 bytes, respectively.
**Why human:** Binary size comparison requires actually running the built binary against a scaffolded project, which requires a compiled `writ` binary in PATH.

#### 2. writ compile . returns helpful error

**Test:** In any directory, run `writ compile .`
**Expected:** Exit 1 with message "'.' is a directory. Use `writ build` to compile a project."
**Why human:** Requires running the CLI binary.

#### 3. writ new "Next steps" message

**Test:** Run `writ new myproject` and read the terminal output.
**Expected:** Step 3 says "Run 'writ build' to compile" (not the old "writ compile sources/main.writ").
**Why human:** Requires running the CLI binary.

These items are low-risk: the code is directly verified at lines 296-303 (`cmd_new`), 481-485 (`cmd_compile` directory check), and the SUMMARY documents a passing smoke test.

## Commit Verification

All four commits claimed in SUMMARYs are present in git history:
- `89000b4` — feat(45-01): add ProfileConfig/ProfilesConfig to config.rs
- `0a2301c` — feat(45-01): thread emit_debug_info through emit pipeline
- `1fd3973` — feat(45-02): implement writ build subcommand with pipeline refactor
- `5d5691b` — feat(45-02): add profile sections to spec (section 2.7 Build Profiles)

## Test Suite Status

All workspace tests pass (0 failures). Config-specific tests: 9/9 pass, including all 3 new profile deserialization tests.

## Gaps Summary

No gaps. All must-haves from both plan frontmatter definitions are verified against the actual codebase. The implementation matches the plan specifications without stubs, placeholders, or missing wiring.

---
_Verified: 2026-03-06_
_Verifier: Claude (gsd-verifier)_
