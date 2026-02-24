# 1. Writ Language Specification
## 3. Naming Conventions & Style Guide

The following naming conventions are enforced by the compiler as warnings and by the language server as suggestions.

| Construct                                         | Convention           | Examples                                  |
|---------------------------------------------------|----------------------|-------------------------------------------|
| Types (struct, entity, enum, contract, component) | PascalCase           | `Merchant`, `QuestStatus`, `Interactable` |
| Enum variants                                     | PascalCase           | `InProgress`, `Completed`, `None`         |
| Functions, methods                                | camelCase            | `calculateDamage`, `getOrCreate`          |
| Variables, parameters                             | camelCase            | `playerName`, `goldAmount`                |
| Constants                                         | SCREAMING_SNAKE_CASE | `MAX_HEALTH`, `DEFAULT_GOLD`              |
| Namespaces                                        | snake_case           | `survival`, `quest_system`                |
| Attributes                                        | PascalCase           | `Singleton`, `Deprecated`, `Locale`       |
| Builtin primitive types                           | lowercase            | `int`, `float`, `bool`, `string`          |
| Builtin generic types                             | PascalCase           | `Option`, `Result`, `List`, `Map`         |

> **Note:** Primitive types (`int`, `float`, `bool`, `string`) are lowercase because they are language keywords. Array
> types use postfix `[]` notation (e.g., `int[]`). Standard library types (`Option`, `Result`, `List`, `Map`, `Set`,
`EntityList`) follow PascalCase as they are regular types, even though `Option` and `Result` have special compiler
> support.

---

