---
phase: 39-extend-method-defs-metadata-to-include-parameter-names-register-indices-and-type-information
plan: 03
subsystem: spec, requirements
tags: [il-spec, requirements, META-01, format_version]

requires:
  - phase: 39 plan 02
    provides: disassembler showing named params, re-blessed golden tests

provides:
  - IL spec §2.16.5 documents param_count in MethodDef table
  - IL spec §2.16.1 documents format version history
  - IL spec §2.16.6 references MethodDef.param_count as authoritative
  - REQUIREMENTS.md META-01 marked complete
  - ROADMAP.md Phase 39 plans all marked [x]
---

## Summary

Updated the IL spec to document the new `MethodDef.param_count` field and format_version 2 change. Marked META-01 complete in REQUIREMENTS.md. Human-verify checkpoint auto-approved (auto_advance: true).

## What Was Built

- `language-spec/spec/45_2_16_il_module_format.md`:
  - §2.16.1: Added format version history note (v1=20-byte MethodDef, v2=24-byte MethodDef)
  - §2.16.5 MethodDef table: `param_count(u16)` added to field list
  - §2.16.5 notes: New `MethodDef.param_count` paragraph explaining the field semantics
  - §2.16.6 register layout: Added sentence: "The `param_count` value is stored explicitly in the `MethodDef` row..."
- `.planning/REQUIREMENTS.md`: META-01 changed from `[ ]` to `[x]`; traceability updated to "Complete"
- `.planning/ROADMAP.md`: All three Phase 39 plans marked `[x]`

## Checkpoint Result

Auto-approved (auto_advance: true in config). Verification confirmed:
- `cargo test -p writ-module && cargo test -p writ-golden`: all pass
- `writ compile fn_typed_params.writ | writ disasm`: shows `(a: int, b: int) -> int`
- Binary bytes 4-5 of compiled .writil: `02 00` (format_version = 2)

## Self-Check: PASSED

All must_haves verified:
- IL spec §2.16.5 MethodDef includes param_count: YES
- IL spec §2.16.6 references MethodDef.param_count: YES
- Binary format row size in spec matches implementation (24 bytes): YES (format_version note says "24 bytes, padded from 22")
- REQUIREMENTS.md has META-01 entry: YES (marked complete)
- Human checkpoint auto-approved: YES (auto_advance: true)
