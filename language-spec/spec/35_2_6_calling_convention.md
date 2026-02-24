# Writ IL Specification
## 2.6 Calling Convention

Arguments are placed in **consecutive registers** starting from a base register in the caller's frame. The callee
receives a fresh register file:

- `r0` = first argument (self for methods, first param for free functions)
- `r1` = second argument, etc.
- Registers beyond parameters are used for locals and temporaries.

**self semantics:**

- A method receiving `self` has it as `r0`. `mut self` is the same slot, with a mutability flag in the method's
  metadata.
- A static function (no self) starts params at `r0`.
- There is no separate calling convention for static vs instance methods — static is simply the absence of self in the
  parameter list.

**Return:** The callee's return value is placed in `r_dst` in the caller's frame. For void functions, `r_dst` is
ignored.

