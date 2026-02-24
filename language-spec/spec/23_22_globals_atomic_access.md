# Writ Language Specification
## 22. Globals & Atomic Access

### 22.1 Global Variables

Global mutable state is declared with `global mut`. Global immutable values use `const`. Globals are visible throughout
their namespace.

```
// Immutable constant (compile-time known)
const MAX_REPUTATION: int = 100;

// Mutable global (requires explicit global mut)
global mut reputation: int = 0;
global mut questLog: Map<string, QuestStatus> = Map::new();
global mut partyMembers: EntityList<Entity> = EntityList::new();
```

### 22.2 Concurrency Safety

All reads and writes to `global mut` variables are implicitly serialized by the runtime. Individual read or write
operations are atomic. No manual locking is required for single-operation access.

```
// These are safe — each is a single atomic operation
reputation += 10;
let currentRep = reputation;
```

### 22.3 Atomic Blocks

For multi-step operations that must execute without interleaving from other tasks, use `atomic { }`. The runtime
guarantees no other task reads or writes the involved globals during an atomic block.

```
atomic {
    let old = reputation;
    reputation = old + bonus;
    if reputation > MAX_REPUTATION {
        reputation = MAX_REPUTATION;
    }
}

// Simple operations don't need atomic
reputation += 10;  // already atomic (single operation)
```

> **Note:** `atomic` blocks should be kept small. Long-running operations inside `atomic` will block all other tasks
> that access the same globals. The runtime may warn on `atomic` blocks that contain yield points (`wait`, `say`, etc.).

---

