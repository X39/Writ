# Writ IL Specification
## 2.13 Save/Load Serialization

**Status:** Resolved.

The runtime must be able to serialize and deserialize the entire VM state for game save/load. The host decides **when**
to serialize or deserialize — this is not a runtime concern. The spec defines **what** must be serializable and *
*recommends** strategies for version compatibility, but the save format and migration policies are runtime concerns.

### 2.13.1 Spec Requirements

The runtime **must** support serializing and restoring the full VM state. The following state constitutes a complete
save:

- All task call stacks (frames: method_id, pc, register values)
- All global variable values
- The full heap (all live GC objects: structs, arrays, strings, closures, delegates)
- The entity registry (all live entities with their script-side field values)
- The task tree (parent-child relationships, scoped vs detached)

The following is explicitly **excluded** from the script save:

- Native/host state (sprites, physics bodies, audio) — the host is responsible for its own save/load
- Extern component field values — these are host-owned (see §2.14)

The runtime must not attempt to serialize while any extern call is in-flight. The suspend-and-confirm model (§2.14.2)
guarantees that the VM is at a well-defined transition point before the host can request a save, but the runtime must
additionally ensure all pending host confirmations have resolved before serializing.

### 2.13.2 Module Versioning

Each compiled IL module carries a **version identifier** (format is runtime-defined — content hash, semantic version, or
both). On deserialization, the runtime compares the saved module version against the currently loaded module.

If a version mismatch is detected, the runtime **must** report the conflict to the host. Behavior beyond reporting is
runtime-defined, but the spec **recommends** the following strategy:

**Recommended: IL coexistence.** The save includes the full IL module binary that was active at save time. On restore,
if the current IL differs, the runtime loads the saved IL alongside the current IL. Existing call stacks continue
executing against the saved (old) IL. As stack frames return, they re-enter the current (new) IL at function call
boundaries. The old IL is discarded once no call stacks reference it.

This approach handles the common case — a mod update or patch between play sessions — without requiring migration logic.
The old code naturally drains out as functions return.

**Limitations the runtime should be aware of:**

- Code that never returns (e.g., `while true { ... }` with no function calls in the body) will run the old IL
  indefinitely. The compiler may warn about such patterns and recommend inserting function call boundaries inside
  long-running loops.
- Game authors building moddable games should provide extern mechanisms like timers or event hooks that allow looping
  behavior to pass through function call boundaries, enabling IL transitions.

### 2.13.3 Extern Calls During Serialization

Serialization must not occur while extern calls are outstanding. Since the suspend-and-confirm model suspends the VM on
extern calls until the host confirms, a well-behaved host will not request a save while it has unconfirmed operations
pending. If it does, the runtime must either defer the save until all pending extern calls resolve, or reject the save
request.

