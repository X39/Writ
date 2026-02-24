# Writ IL Specification
## 2.1 Register-Based Virtual Machine

**Decision:** Register-based (not stack-based).

**Rationale:**

- All execution state is explicit: each call frame = `(method_id, pc, registers[])`.
- Serialization is straightforward — walk the stack, dump each frame's registers + pc.
- Virtual registers (unlimited per function, numbered sequentially) avoid the complexity of physical register
  allocation. The compiler assigns registers linearly.
- Better fit for JIT compilation (closer to machine register model).

**Implications:**

- Instructions encode source/destination registers explicitly.
- Arguments to calls occupy consecutive registers (compiler arranges this).
- Each function declares its register count in the method body header.

