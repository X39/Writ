# Writ IL Specification
## 2.8 Serialization Critical Sections — REMOVED

The original design proposed `CRITICAL_BEGIN` / `CRITICAL_END` instructions to mark code regions where serialization
must not occur. This is no longer necessary: the **suspend-and-confirm model** (§2.14.2) ensures the runtime only
serializes at well-defined transition points (host call boundaries, yield points). Since the VM is never serialized
mid-instruction or mid-expression, there is no need for explicit critical sections.

Native resources (OS handles, GPU state) are the host's responsibility and are never included in script saves.

