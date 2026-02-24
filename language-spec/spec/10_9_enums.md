# 1. Writ Language Specification
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
    None,
    Some(value: T),
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

### 9.3 Patterns

Patterns appear in `match` arms and `if let` bindings. Writ supports seven pattern forms.

#### 9.3.1 Literal Patterns

Integer, string, boolean, and null literals match by value.

```
match command {
    42 => { handleSpecial(); }
    "quit" => { exit(); }
    true => { enable(); }
    null => { handleMissing(); }
}
```

#### 9.3.2 Wildcard

`_` matches any value and discards it. Commonly used as a catch-all arm.

```
match status {
    QuestStatus::Completed => { reward(player); }
    _ => { log("Not complete yet"); }
}
```

#### 9.3.3 Variable Binding

A bare identifier matches any value and binds it to a new variable within the arm body.

```
match damage {
    0 => { log("Miss!"); }
    amount => { log($"Took {amount} damage"); }
}
```

#### 9.3.4 Enum Destructuring

Qualified enum variants with parenthesized sub-patterns destructure variant payloads.

```
match result {
    Result::Ok(val) => { use(val); }
    Result::Err(err) => { log(err.message); }
}
```

#### 9.3.5 Nested Destructuring

Patterns nest arbitrarily — sub-patterns can themselves be enum destructuring, wildcards, literals, or bindings.

```
match result {
    Result::Ok(QuestStatus::InProgress(step)) => { log($"On step {step}"); }
    Result::Ok(QuestStatus::Completed) => { reward(player); }
    Result::Ok(_) => { log("Other OK status"); }
    Result::Err(err) => { log(err.message); }
}
```

#### 9.3.6 Or-Patterns

Multiple patterns separated by `|` share a single arm body.

```
match status {
    QuestStatus::Completed | QuestStatus::Failed(_) => { removeFromActive(); }
    _ => { log("Still in progress"); }
}
```

#### 9.3.7 Range Patterns

Inclusive ranges (`..=`) match any value within the range. Useful for numeric thresholds.

```
match health {
    0 => { die(); }
    1..=25 => { log("Critical!"); }
    26..=75 => { log("Wounded"); }
    _ => { log("Healthy"); }
}
```

---

