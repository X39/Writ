---
created: "2026-03-29T20:00:00.000Z"
title: Default contract method implementations
area: compiler
files: []
priority: high
---

Add support for default method bodies in contract declarations (spec, parser, AST, type checker, emitter, runtime dispatch fallback). Needed for `Iterable<T>.contains()` default impl and future contract ergonomics.

**Scope:** Parser extends ContractMember to support optional bodies; compiler emits default method MethodDef entries; runtime dispatch falls back to default if no explicit impl found.

**Blocked by:** Nothing — can be done independently.
**Enables:** `contains` as a default Iterable<T> method, general contract default methods.
