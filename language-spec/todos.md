# Implement `writ.toml` handling
#### ToDo
Right now, compile does not handle compilation from writ.toml. Fix that by implementing it.
We have to support passing a  writ.toml, a directory or .writ file to modules.
We also have to revisit how configurations work, making them more "debug" and "release" like (and encompass them into the bin/configuration/artifacts structure correctly).
# `writ-golden/tests/golden/fn_log_say_choice.writc` is severly broken
#### ToDo
Figure out what is broken by checking against the spec and fix it.
After that: Check whether `tools/writ_module.hexpat` parses the artifact in ImHex again and, if not, fix the problem in said file too.

# Choice options have to be renamed to not conflict with Option enum
#### ToDo
Right now, the spec demands the Option enum and has the Option type (or method, i am actually not sure right now).
That is a name conflict. Add a ChoiceOption type instead to avoid this.
Update the spec to reflect this and update the inbuilt module.

# Extend log method or make it a namespace/type with sub-methods
#### ToDo
Right now, the log method does not allow to do a multitude of things, and works more like a "println" function.
Log should, however, allow multiple log levels, optional categories and other things (eg. value logging), to make it useful.
Update the spec to reflect this and fix the inbuilt log method.

# Inbuilt methods need to be referenced by their full path
#### ToDo
Update the spec to be more clear that inbuilt methods live in no namespace and are always available.

#### Works
```writ
pub fn main() {
    ::log("saying Test");
    ::say("Test");
    ::log("showing choice");
    ::choice([
            ::Option("Good!", fn() {
                ::say(Option::None, "Glad to hear it.");
            }),
            ::Option("Not great", fn() {
                ::say(Option::None, "Things are rough.");
                ::say(Option::None, "Sorry to hear that.");
            }),
        ]);
}
```

#### Does not work
```writ
pub fn main() {
    log("saying Test");
    say("Test");
    log("showing choice");
    choice([
            Option("Good!", fn() {
                say(Option::None, "Glad to hear it.");
            }),
            Option("Not great", fn() {
                say(Option::None, "Things are rough.");
                say(Option::None, "Sorry to hear that.");
            }),
        ]);
}
```

# Enums cannot be used as rust enums can

#### ToDo
Update spec and lowering to allow enum variants to be imported and update the spec to expose Option<T> enum members by default.
#### Works
```writ
fn main() {
    let opt_val_keyword: bool? = null;
    let opt_val_type: Option<bool> = Option::None;
    let opt_implicit = Option::Some(true);
    let opt_call_some = produce_option_some();
    let opt_call_none = produce_option_none();
}

fn produce_option_some() -> Option<bool> {
    Option::Some(true)
}

fn produce_option_none() -> Option<bool> {
    Option::None
}
```

#### Does not work
```writ
fn main() {
    let opt_val_keyword: bool? = None;
    let opt_val_type: Option<bool> = None;
    let opt_implicit = Some(bool);
    let opt_call_some = produce_option_some();
    let opt_call_none = produce_option_none();
}

fn produce_option_some() -> Option<bool> {
    Some(true)
}

fn produce_option_none() -> Option<bool> {
    None
}
```