# Writ IL Specification
## 2.10 Self Parameter

**Decision:** Methods take explicit `self` or `mut self` as their first parameter. This is now specified in the language
spec (§12.5).

- `self` — immutable receiver. Cannot modify fields or call `mut self` methods through `self`.
- `mut self` — mutable receiver. Can read and modify fields, call any method through `self`.
- Absence of `self` — static function (no receiver).

**IL mapping:**

- `self` is always `r0` in the callee's register file (see §2.6).
- The method's metadata carries a mutability flag: `is_mut_self: bool`.
- The runtime enforces that `mut self` methods are only called through mutable bindings (or the compiler enforces this
  statically — either is valid).
- Operator methods have implicit `self` with mutability determined by operator kind: all read operators are immutable,
  `[]=` is mutable.
- Lifecycle hooks (`on create`, `on interact`, `on destroy`) have implicit `mut self`.

