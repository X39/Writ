# Your First Dialogue

Writ's flagship feature is `dlg` -- dialogue blocks where plain text is the default and code requires explicit escaping. This tutorial walks through building dialogue from basic lines to branching conversations.

## A Simple Greeting

Dialogue blocks use `@` to attribute lines to a speaker:

```writ
entity Narrator {}

dlg greet {
    @Narrator Hello, traveler.
    @Narrator Welcome to the village.
}

pub fn main() {
    greet();
}
```

`dlg` blocks compile down to regular functions. Calling `greet()` from `main` runs the dialogue.

Each `@Narrator` line becomes a `say()` call at runtime -- the host engine displays the text and waits for the player to advance before continuing.

## Multi-line Speaker Blocks

Setting `@speaker` on its own line makes it the active speaker for all following lines:

```writ
dlg introduction {
    @Narrator
    The sun rises over the valley.
    Birds sing in the distance.
    A figure approaches on the road.
}
```

All three lines are attributed to the Narrator without repeating `@Narrator` each time.

## Code Escapes with $

The `$` sigil escapes from dialogue into code. Use it to run logic between dialogue lines:

```writ
fn format_greeting(name: string) -> string {
    "Welcome, " + name + "!"
}

dlg greet(npc: Entity, player_name: string) {
    @npc Hello, traveler.
    $ let greeting: string = format_greeting(player_name);
    @npc Great to meet you!
    $ let reward: int = 10 * 3;
    $ ::log::info("Computed reward");
}
```

`$` works for single statements. For multiple statements, use a block:

```writ
dlg scene {
    @Narrator Let me check your reputation.
    $ {
        let rep = getReputation();
        let mut adjusted = rep * modifier;
        if adjusted > 100 {
            unlockAchievement("famous");
        }
    }
    @Narrator Interesting...
}
```

## String Interpolation

Dialogue text supports `{expr}` interpolation without any prefix:

```writ
dlg greet(playerName: string) {
    @Narrator Hello, {playerName}. You have {getGold()} gold.
}
```

## Choices

`$ choice` presents options to the player. Each option has a quoted label and a dialogue block:

```writ
dlg shopkeeper {
    @OldTim
    What would you like?
    $ choice {
        "Buy something" {
            Let me show you my wares.
            $ openShopUI();
        }
        "Just looking" {
            Take your time.
        }
        "Goodbye" {
            Farewell, traveler.
        }
    }
}
```

The runtime suspends on `$ choice` and waits for the host to report which option the player selected.

## Conditional Dialogue

Use `$ if` and `$ match` for branching dialogue. The condition is code, but the branches stay in dialogue context:

```writ
dlg greet(reputation: int) {
    @Narrator
    $ if reputation > 50 {
        You're quite famous around here.
    } else {
        I don't think I know you.
    }
    Either way, I have a task for you.
}
```

`$ match` works with enums:

```writ
dlg questUpdate(status: QuestStatus) {
    @Narrator
    $ match status {
        QuestStatus::NotStarted => {
            I have a task for you, adventurer.
        }
        QuestStatus::Active => {
            How's that task coming along?
        }
        QuestStatus::Completed => {
            Well done! Here's your reward.
            $ giveReward();
        }
    }
}
```

## Transitions

The `->` operator performs a terminal transition to another dialogue -- like a tail call:

```writ
dlg questIntro {
    @Narrator A great evil threatens the land.
    $ choice {
        "Tell me more" {
            -> questDetails
        }
        "Not interested" {
            @Narrator Very well. Perhaps another time.
            -> townSquare
        }
    }
}
```

Transitions can pass arguments:

```writ
dlg shopEntry(player: Entity) {
    @Narrator You enter the shop.
    -> shopDialog(player)
}
```

```admonish note
`->` is terminal -- execution does not return after a transition. For non-terminal dialogue calls, use `$ questDetails();` instead.
```

## Putting It Together

Here is a complete dialogue scene with speakers, code escapes, choices, and transitions:

```writ
entity Merchant {
    name: string,
    has_quest: bool,
}

dlg merchant_greeting(merchant: Entity) {
    @merchant Welcome to my shop, traveler!
    @merchant I have something for you.
    -> merchant_quest(merchant)
}

dlg merchant_quest(merchant: Entity) {
    $ let state: QuestState = QuestState::NotStarted;
    $ let available: bool = can_offer_quest(state);
    $ if available {
        @merchant I have a task for you.
        @merchant Retrieve the ancient scroll from the ruins.
    } else {
        @merchant You already have a task. Come back later.
    }
}
```

For the full dialogue syntax reference, see [Dialogue Blocks](../language-ref/dialogue.md).
