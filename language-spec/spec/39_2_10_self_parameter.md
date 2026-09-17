# 2.10 Self Parameter

**Decision:** Methods take explicit `self` or `mut self` as their first parameter. This is now specified in the language
spec (§12.5).

- `self` — immutable receiver. Cannot modify fields or call `mut self` methods through `self`.
- `mut self` — mutable receiver. Can read fields, modify fields declared `mut`, and call any method through `self`.
- Absence of `self` — static function (no receiver).

**IL mapping:**

- `self` is always `r0` in the callee's register file (see §2.6).
- The method's metadata carries a mutability flag: `is_mut_self: bool`.
- The compiler requires the receiver of a `mut self` method call to be a mutable place, such as a `mut` binding or
  mutable receiver path. Immutable bindings, immutable values, and plain `self` cannot call a `mut self` method.
- This is a source-level rule. Method metadata records that the callee uses `mut self`, but the current call
  instructions do not encode whether the caller's receiver register came from a mutable source place. The runtime
  therefore cannot reconstruct that source fact from forged IL; it still independently enforces runtime-checkable
  rules such as read-only FieldDef metadata.
- Operator methods have implicit `self` with mutability determined by operator kind: all read operators are immutable,
  `[]=` is mutable.
- Lifecycle hooks (`on create`, `on interact`, `on destroy`) have implicit `mut self`.

