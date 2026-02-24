# Writ Language Specification
## 9. Enums

Enums are tagged unions. Each variant can optionally carry named data fields. Pattern matching via `match` is
exhaustive — the compiler enforces that all variants are handled.

```
enum QuestStatus {
    NotStarted,
    InProgress(currentStep: int),
    Completed,
    Failed(reason: string),
}

fn describeQuest(status: QuestStatus) -> string {
    match status {
        QuestStatus::NotStarted => { "Not yet begun." }
        QuestStatus::InProgress(step) => { $"On step {step}" }
        QuestStatus::Completed => { "Done!" }
        QuestStatus::Failed(reason) => { "Failed: " + reason }
    }
}
```

### 9.1 Builtin Enums

The following enums are compiler-known and have special syntax support:

```
// Option<T> — nullable values. T? is sugar for Option<T>.
enum Option<T> {
    Some(value: T),
    None,
}

// Result<T, E> — fallible operations. E must implement Error.
enum Result<T, E: Error> {
    Ok(value: T),
    Err(error: E),
}
```

### 9.2 if let (Optional Pattern Matching)

The `if let` construct provides a concise way to match a single pattern, particularly useful for `Option` and
single-variant checks.

```
if let Option::Some(healer) = party.healer {
    healer.heal(player);
}

// With else branch:
if let Option::Some(quest) = activeQuest {
    showQuestUI(quest);
} else {
    showNoQuestMessage();
}
```

---

