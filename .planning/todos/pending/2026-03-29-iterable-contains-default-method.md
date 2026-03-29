---
created: "2026-03-29T20:00:00.000Z"
title: Move contains to Iterable<T> as default method
area: stdlib
files: []
priority: high
---

Once default contract methods are implemented, add `contains(item: T) -> bool` as a default method on `Iterable<T>`. This replaces the removed `ArrayContains` opcode (v14.0). Every Iterable gets `.contains()` for free; explicit impl specialization for arrays can be added later for performance.

**Depends on:** Default contract method implementations todo.
