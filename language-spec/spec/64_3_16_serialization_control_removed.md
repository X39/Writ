# Writ IL Specification
## 3.16 Serialization Control — REMOVED

The `CRITICAL_BEGIN` / `CRITICAL_END` instructions have been removed. The suspend-and-confirm model (§2.14.2 in design
decisions) ensures serialization only occurs at transition points, making explicit critical sections unnecessary.

