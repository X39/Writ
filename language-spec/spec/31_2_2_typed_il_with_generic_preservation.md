# Writ IL Specification
## 2.2 Typed IL with Generic Preservation

**Decision:** The IL preserves full type information. Generics are not monomorphized at compile time.

**Rationale:**

- Enables runtime reflection (planned future feature).
- JIT can monomorphize hot paths selectively.
- Matches CLR model: open generic types in metadata, closed instantiations via TypeSpec.

**Implications:**

- Every register slot has a type identity (known from the method's local type table or inferred from the instruction).
- The runtime carries type tags for dynamic dispatch.
- Metadata tables must represent generic parameters, constraints, and instantiations.
- `(type_tag, contract_id, method_slot) → code_offset` dispatch tables.

