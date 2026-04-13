# Building a Quest System

This tutorial builds a quest tracking system that ties together enums, pattern matching, arrays, optionals, globals, atomic blocks, and dialogue. By the end you will have a working quest manager that demonstrates how Writ's features combine in a real game system.

## Defining Quest State

Start with enums for quest status and type:

```writ
enum QuestStatus {
    NotStarted,
    Active,
    Completed,
    Failed,
}

enum QuestType {
    MainStory,
    SideQuest,
    Daily,
}
```

## Constants and Globals

Constants define system limits. Globals track shared mutable state -- they require `global mut` and are accessed atomically when needed:

```writ
const MAX_QUESTS: int = 10;
const XP_MULTIPLIER: int = 2;

global mut active_quest_count: int = 0;
global mut total_xp: int = 0;
```

## Quest Logic Functions

Write pure functions for quest logic. Pattern matching with `match` handles each enum variant:

```writ
fn calculate_reward(base_xp: int, quest_type: QuestType) -> int {
    let multiplier: int = match quest_type {
        QuestType::MainStory => { XP_MULTIPLIER * 3 }
        QuestType::SideQuest => { XP_MULTIPLIER }
        QuestType::Daily => { 1 }
    };
    base_xp * multiplier
}

fn is_quest_available(status: QuestStatus) -> bool {
    match status {
        QuestStatus::NotStarted => { true }
        QuestStatus::Active => { false }
        QuestStatus::Completed => { false }
        QuestStatus::Failed => { true }
    }
}
```

## Working with Arrays and Optionals

Search through a quest log array, returning `Option` for results that may not exist:

```writ
fn find_first_active(statuses: QuestStatus[]) -> int? {
    let mut idx: int = 0;
    for s in statuses {
        match s {
            QuestStatus::Active => { return Option::Some(idx); }
            QuestStatus::NotStarted => { idx = idx + 1; }
            QuestStatus::Completed => { idx = idx + 1; }
            QuestStatus::Failed => { idx = idx + 1; }
        }
    }
    Option::None
}
```

`int?` is shorthand for `Option<int>`. The function returns `Option::Some(idx)` when found, or `Option::None` at the end.

## Atomic Global Updates

When modifying multiple globals that must stay consistent, wrap them in an `atomic` block:

```writ
fn complete_quest(status: QuestStatus, base_xp: int, quest_type: QuestType) -> QuestStatus {
    match status {
        QuestStatus::Active => {
            let reward: int = calculate_reward(base_xp, quest_type);
            atomic {
                total_xp = total_xp + reward;
                active_quest_count = active_quest_count + 1;
            }
            QuestStatus::Completed
        }
        QuestStatus::NotStarted => { QuestStatus::Failed }
        QuestStatus::Completed => { QuestStatus::Completed }
        QuestStatus::Failed => { QuestStatus::Failed }
    }
}
```

## Dialogue Integration

Connect quest logic to the dialogue system using entities and `say`:

```writ
entity Narrator {}

fn announce_quest(speaker: Entity, quest_type: QuestType) {
    match quest_type {
        QuestType::MainStory => {
            ::log::info("Announcing main story quest");
            ::say(speaker, "A new chapter of the main story begins.");
        }
        QuestType::SideQuest => {
            ::log::info("Announcing side quest");
            ::say(speaker, "A side quest has appeared.");
        }
        QuestType::Daily => {
            ::log::info("Announcing daily quest");
            ::say(speaker, "Your daily quest is ready.");
        }
    }
}
```

## Putting It All Together

The `main` function wires everything together -- initialization, dialogue, optional handling, and cleanup:

```writ
fn main() {
    defer {
        ::log::info("Quest system session ended.");
    }

    let speaker: Entity = Entity.getOrCreate<Narrator>();

    // Initialise status and type arrays
    let statuses: QuestStatus[] = [
        QuestStatus::NotStarted,
        QuestStatus::Active,
        QuestStatus::Completed,
        QuestStatus::Failed,
    ];
    let types: QuestType[] = [
        QuestType::MainStory,
        QuestType::SideQuest,
        QuestType::Daily,
    ];

    // Announce each quest type
    for t in types {
        announce_quest(speaker, t);
    }

    // Find and process the first active quest
    let maybe_active: int? = find_first_active(statuses);
    match maybe_active {
        Option::None => {
            ::say(speaker, "No quests are currently active.");
        }
        Option::Some(idx) => {
            ::say(speaker, "You have an active quest.");
            let new_status: QuestStatus = complete_quest(
                QuestStatus::Active, 50, QuestType::MainStory
            );
            match new_status {
                QuestStatus::Completed => { ::log::info("Quest completed."); }
                QuestStatus::Failed => { ::log::info("Quest failed."); }
                QuestStatus::Active => { ::log::info("Quest still active."); }
                QuestStatus::NotStarted => { ::log::info("Quest not started."); }
            }
        }
    }

    // Guard: only continue if within capacity
    if active_quest_count < MAX_QUESTS {
        ::say(speaker, "Quest tracker is ready.");
    }

    // Atomic snapshot of session state
    atomic {
        let snapshot_xp: int = total_xp;
        let snapshot_count: int = active_quest_count;
    }
}
```

Note the `defer` block at the top -- it runs when `main` exits regardless of how it exits, making it useful for cleanup logging and resource release.

## What This Demonstrates

| Feature | Where it appears |
|---------|-----------------|
| Enums and match | `QuestStatus`, `QuestType`, all `match` blocks |
| Functions | `calculate_reward`, `is_quest_available`, `find_first_active` |
| Arrays and iteration | `QuestStatus[]`, `for s in statuses` |
| Optionals (`T?`) | `find_first_active` return type, `Option::Some`/`Option::None` |
| Constants | `MAX_QUESTS`, `XP_MULTIPLIER` |
| Global mutable state | `active_quest_count`, `total_xp` |
| Atomic blocks | Global XP/count updates, session snapshot |
| Defer | Cleanup logging in `main` |
| Entities and singletons | `Narrator`, `Entity.getOrCreate` |
| Dialogue builtins | `::say`, `::log::info` |
