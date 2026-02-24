# Writ Language Specification
## 18. Error Handling

Writ separates error handling into distinct, non-overlapping mechanisms:

| Operator | Works On           | Behavior                          | Use When                   |
|----------|--------------------|-----------------------------------|----------------------------|
| `?`      | `Option<T>` / `T?` | Propagates `None`, unwraps `Some` | Chaining nullable access   |
| `!`      | Option and Result  | Unwraps or crashes the task       | Confident the value exists |
| `try`    | `Result<T, E>`     | Propagates `Err`, unwraps `Ok`    | Bubbling errors up in `fn` |
| `match`  | Both               | Explicit pattern matching         | Need to handle both paths  |

### 18.1 The ? Operator (Null Propagation)

Works exclusively on `Option<T>`. The containing function must return an `Option` type. Does NOT work on `Result`.

```
fn getHealerName(p: Party) -> string? {
    p.healer?.name   // if healer is None, returns None
}
```

### 18.2 The ! Operator (Unwrap or Crash)

Works on both Option and Result. Extracts the value or terminates the current task with a runtime error. In `dlg`
blocks, the runtime catches the crash and handles it (logging, fallback dialogue, etc.).

```
let healer = party.healer!;      // crash if None
let quest = loadQuest(id)!;      // crash if Err
```

### 18.3 The try Keyword (Error Propagation)

Works exclusively on `Result<T, E>`. The containing function must return a compatible Result. Does NOT work on Option.

```
fn loadBothQuests() -> Result<QuestPair, Error> {
    let a = try loadQuest("main_01");   // propagates Err
    let b = try loadQuest("side_03");   // propagates Err
    Result::Ok(QuestPair(a, b))
}
```

### 18.4 The Error Contract

```
contract Error {
    fn message() -> string;
}

struct QuestError {
    code: int,
    detail: string,
}

impl Error for QuestError {
    fn message() -> string {
        $"Quest error {self.code}: {self.detail}"
    }
}
```

### 18.5 Separation Summary

The `?` and `try` operators occupy entirely separate domains. There is no implicit conversion between Option and Result.
This eliminates a class of subtle bugs and keeps the mental model simple: `?` is for null, `try` is for errors, `!` is
for "I'm sure."

---

