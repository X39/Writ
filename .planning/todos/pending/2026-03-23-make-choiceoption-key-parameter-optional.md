---
created: 2026-03-23T21:36:30.593Z
title: Make ChoiceOption key parameter optional
area: compiler
files:
  - language-spec/spec/28_27_standard_library_builtins.md:49
  - writ-compiler/src/lower/dialogue.rs:637-672
  - writ-compiler/src/emit/collect/builtins.rs:82-87
  - writ-compiler/src/check/env.rs:260
---

## Problem

`ChoiceOption(label: string, key: string, body: fn() -> void) -> ChoiceOption` requires a localization key as a mandatory parameter. This is awkward for user code calling `ChoiceOption` directly from `fn` contexts where localization isn't needed. The `key` parameter feels informal as a required positional arg — users shouldn't need to pass a loc key just to create a simple choice.

The compiler's `dlg` lowering always auto-generates an FNV-1a loc key, so the parameter is always populated in lowered code. But for direct user calls, requiring it is unnecessary friction.

## Solution

Preferred approach: **mirror the `say`/`say_localized` split** already in the language:

- `ChoiceOption(label: string, body: fn() -> void) -> ChoiceOption` — no loc key (simple form)
- `ChoiceOptionLocalized(label: string, key: string, body: fn() -> void) -> ChoiceOption` — with loc key

This keeps the API consistent with the existing `say`/`say_localized` pattern. The `dlg` lowering would emit `ChoiceOptionLocalized` when a manual `#key` is present, and plain `ChoiceOption` otherwise.

Alternative: move key to end as `string?` — `ChoiceOption(label, body, key?)` — simpler change but less consistent with existing patterns.

Changes needed: spec (§1.27.4), compiler lowering (`dialogue.rs`), builtin injection (`builtins.rs`, `env.rs`), ExternDef signatures, and runtime extern dispatch.
