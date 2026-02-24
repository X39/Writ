# Writ IL Specification
## 2.7 Operator Dispatch

Operators on primitive types use dedicated IL instructions (`ADD_I`, `CMP_LT_F`, etc.) — these are the fast path with no
dispatch overhead.

Operators on user-defined types are lowered by the compiler to `CALL_VIRT` through the corresponding contract (`Add`,
`Sub`, `Eq`, `Ord`, `Index`, etc.). The IL does not have separate "overloaded operator" instructions — the contract
dispatch system handles it uniformly.

This is a **compiler concern**, not an IL concern. The compiler knows the types at emit time and selects the appropriate
instruction.

