//! DAP wire-protocol integration tests for fn_typed_params.writ.
//!
//! Uses a bidirectional DapClient (pipe-based) to exchange messages with a real
//! DapServer, enabling dynamic extraction of threadId, frameId, and
//! variablesReference from responses instead of hardcoding them.

mod common;

use common::{workspace_file, DapClient};
use serde_json::json;

const FIXTURE: &str = "writ-golden/tests/golden/fn_typed_params.writ";

/// Test 1: Initialize returns expected capabilities and initialized event is sent.
#[test]
fn test_initialize_capabilities() {
    let mut client = DapClient::start();

    let resp = client.initialize();

    // Verify body.supportsConfigurationDoneRequest = true
    let body = resp.get("body").cloned().unwrap_or(json!({}));
    assert_eq!(
        body.get("supportsConfigurationDoneRequest")
            .and_then(|v| v.as_bool()),
        Some(true),
        "initialize body should contain supportsConfigurationDoneRequest=true, got: {}",
        body
    );

    client.shutdown();
}

/// Test 2: Launch and run to completion without breakpoints.
#[test]
fn test_launch_and_run_to_completion() {
    let fixture_path = workspace_file(FIXTURE);
    let mut client = DapClient::start();

    client.initialize();
    client.configuration_done();

    let (launch_resp, events) = client.launch(&fixture_path, false);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );

    // No breakpoints → program runs to completion → terminated event
    let has_terminated = events
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("terminated"));
    assert!(
        has_terminated,
        "should receive terminated event after program completes, events: {:?}",
        events
            .iter()
            .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
            .collect::<Vec<_>>()
    );

    client.shutdown();
}

/// Test 3: Breakpoint hit and full variable inspection chain.
///
/// Sets a breakpoint on line 12, launches, then follows the dynamic value chain:
///   stopped event → threadId → stackTrace → frameId → scopes → variablesReference → variables
///
/// Line 12 (`let flag: bool = is_positive(x);`) is chosen so that `x` is already
/// in scope (assigned on line 11), verifying both variable presence and name filtering.
#[test]
fn test_breakpoint_hit_and_inspect() {
    let fixture_path = workspace_file(FIXTURE);
    let mut client = DapClient::start();

    client.initialize();

    // Set breakpoint on line 12: `let flag: bool = is_positive(x);`
    // At this point x is already assigned (line 11 completed), so x is in scope.
    let bp_body = client.set_breakpoints(&fixture_path, &[12]);
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(|v| v.as_array())
        .expect("should have breakpoints array");
    assert_eq!(breakpoints.len(), 1, "should have 1 breakpoint entry");

    client.configuration_done();

    // Launch — should stop at breakpoint
    let (launch_resp, events) = client.launch(&fixture_path, false);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );

    // Extract stopped event
    let stopped = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"))
        .expect("should receive stopped event at breakpoint");

    let stopped_body = stopped.get("body").cloned().unwrap_or(json!({}));
    assert_eq!(
        stopped_body.get("reason").and_then(|v| v.as_str()),
        Some("breakpoint"),
        "stopped reason should be 'breakpoint', got: {:?}",
        stopped_body.get("reason")
    );

    // Extract dynamic threadId from stopped event
    let thread_id = stopped_body
        .get("threadId")
        .and_then(|v| v.as_i64())
        .expect("stopped event should have threadId");

    // Verify threadId appears in threads list
    let threads_body = client.threads();
    let threads = threads_body
        .get("threads")
        .and_then(|v| v.as_array())
        .expect("threads body should have threads array");
    assert!(
        !threads.is_empty(),
        "should have at least 1 thread while stopped"
    );
    assert!(
        threads.iter().any(|t| t.get("id").and_then(|v| v.as_i64()) == Some(thread_id)),
        "thread_id {} should appear in threads list: {:?}",
        thread_id,
        threads
    );

    // StackTrace → extract frameId
    let stack_body = client.stack_trace(thread_id);
    let frames = stack_body
        .get("stackFrames")
        .and_then(|v| v.as_array())
        .expect("stackTrace should have stackFrames array");
    assert!(
        !frames.is_empty(),
        "should have at least 1 stack frame while stopped"
    );

    let top_frame = &frames[0];
    let frame_id = top_frame
        .get("id")
        .and_then(|v| v.as_i64())
        .expect("top frame should have id");

    // Verify breakpoint line
    let top_line = top_frame
        .get("line")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    assert_eq!(
        top_line, 12,
        "top frame should be at breakpoint line 12, got: {}",
        top_line
    );

    // Scopes → extract variablesReference
    let scopes_body = client.scopes(frame_id);
    let scopes = scopes_body
        .get("scopes")
        .and_then(|v| v.as_array())
        .expect("scopes should have scopes array");
    assert!(
        !scopes.is_empty(),
        "should have at least 1 scope"
    );

    let vars_ref = scopes[0]
        .get("variablesReference")
        .and_then(|v| v.as_i64())
        .expect("scope should have variablesReference");

    assert_ne!(
        vars_ref, 0,
        "variablesReference must be non-zero (0 means 'no children' in DAP), got: {}",
        vars_ref
    );

    // Variables — use the dynamic variablesReference
    let vars_body = client.variables(vars_ref);
    let variables = vars_body
        .get("variables")
        .and_then(|v| v.as_array())
        .expect("variables should have variables array");
    assert!(
        !variables.is_empty(),
        "should have at least 1 variable in scope"
    );

    // Regression: no variable should have an empty name (unnamed temporaries must be filtered)
    for var in variables {
        let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!name.is_empty(), "variable should have a non-empty name, got: {}", var);
    }

    // Continue → should terminate
    let (_cont_resp, cont_events) = client.continue_(thread_id);
    let has_terminated = cont_events
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("terminated"));
    assert!(
        has_terminated,
        "should receive terminated event after continue, events: {:?}",
        cont_events
            .iter()
            .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
            .collect::<Vec<_>>()
    );

    client.shutdown();
}

/// Test 4: Stop on entry emits a stopped event with reason "entry".
#[test]
fn test_stop_on_entry() {
    let fixture_path = workspace_file(FIXTURE);
    let mut client = DapClient::start();

    client.initialize();
    client.configuration_done();

    let (launch_resp, events) = client.launch(&fixture_path, true);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );

    // Extract stopped event
    let stopped = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"))
        .expect("should receive stopped event when stopOnEntry=true");

    let reason = stopped
        .get("body")
        .and_then(|b| b.get("reason"))
        .and_then(|r| r.as_str())
        .unwrap_or("");
    assert_eq!(
        reason, "entry",
        "stopped reason should be 'entry', got: {}",
        reason
    );

    client.shutdown();
}

/// Test 5: VS Code ordering — launch arrives before configurationDone.
/// Execution must be deferred until configurationDone, so breakpoints set
/// between launch and configurationDone are still respected.
#[test]
fn test_launch_before_configuration_done_with_breakpoint() {
    let fixture_path = workspace_file(FIXTURE);
    let mut client = DapClient::start();

    client.initialize();

    // Set breakpoint BEFORE launch (like VS Code does)
    let bp_body = client.set_breakpoints(&fixture_path, &[11]);
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(|v| v.as_array())
        .expect("should have breakpoints array");
    assert_eq!(breakpoints.len(), 1);

    // Launch BEFORE configurationDone (VS Code ordering)
    let launch_seq = client.send("launch", json!({
        "program": fixture_path,
        "stopOnEntry": false
    }));
    let (launch_resp, _pre_events) = client.recv_response(launch_seq);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );
    // No execution yet — server is waiting for configurationDone.

    // Now send configurationDone — this triggers execution.
    let cd_seq = client.send_no_args("configurationDone");
    let (cd_resp, _) = client.recv_response(cd_seq);
    assert_eq!(
        cd_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
    );

    // Execution starts now — read the stopped/terminated event.
    let post_events = client.recv_execution_event();
    let stopped = post_events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"));
    assert!(
        stopped.is_some(),
        "should hit breakpoint after deferred execution, events: {:?}",
        post_events.iter()
            .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
            .collect::<Vec<_>>()
    );

    let stopped_body = stopped.unwrap().get("body").cloned().unwrap_or(json!({}));
    assert_eq!(
        stopped_body.get("reason").and_then(|v| v.as_str()),
        Some("breakpoint"),
    );

    client.shutdown();
}

/// Test 6: Launch without a "program" argument returns an error response.
#[test]
fn test_launch_error_missing_program() {
    let mut client = DapClient::start();

    client.initialize();
    client.configuration_done();

    let (launch_resp, _events) = client.launch_raw(json!({
        "stopOnEntry": false
        // "program" intentionally omitted
    }));

    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(false),
        "launch without program should fail, got: {}",
        launch_resp
    );

    let message = launch_resp
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        message.contains("program"),
        "error message should mention 'program', got: '{}'",
        message
    );

    client.shutdown();
}

/// Test 7: Crash halt inspection (break-before-unwind) -- live stack frames + variables after
/// unwrap on None. The scheduler suspends the task with SuspendReason::CrashPending before
/// unwinding, so the DAP server can inspect real stack frames and registers at the crash point.
///
/// Full flow:
///   launch crash_unwrap_none.writ → stopped(exception) [via CrashPending, live stack]
///   → threads → stackTrace [live frames, not CrashInfo snapshots] → scopes/variables
///   → Continue [triggers deferred unwind] → terminated+exited(1)
#[test]
fn test_halt_on_crash_inspect() {
    const FIXTURE_CRASH: &str = "writ-golden/tests/golden/crash_unwrap_none.writ";
    let fixture_path = workspace_file(FIXTURE_CRASH);
    let mut client = DapClient::start();

    client.initialize();
    client.configuration_done();

    // Launch — program crashes at unwrap on None → stopped(exception) event
    let (launch_resp, events) = client.launch(&fixture_path, false);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );

    // Find the stopped event
    let stopped = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"))
        .expect("should receive stopped event after crash");

    let stopped_body = stopped.get("body").cloned().unwrap_or(json!({}));
    assert_eq!(
        stopped_body.get("reason").and_then(|v| v.as_str()),
        Some("exception"),
        "stopped reason should be 'exception' for a crash, got: {:?}",
        stopped_body.get("reason")
    );

    // Extract threadId from the stopped event
    let thread_id = stopped_body
        .get("threadId")
        .and_then(|v| v.as_i64())
        .expect("stopped event should have threadId");

    // Verify an output event with the crash message was emitted
    let has_crash_output = events.iter().any(|e| {
        if e.get("event").and_then(|v| v.as_str()) != Some("output") {
            return false;
        }
        let body = e.get("body").cloned().unwrap_or(json!({}));
        let category = body.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let output = body.get("output").and_then(|v| v.as_str()).unwrap_or("");
        category == "stderr" && (output.contains("Runtime crash") || output.contains("crash") || output.contains("unwrap"))
    });
    assert!(
        has_crash_output,
        "should receive stderr output event with crash message, events: {:?}",
        events
            .iter()
            .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some("output"))
            .collect::<Vec<_>>()
    );

    // Threads — should return the crashed thread with matching id and non-"terminated" name
    let threads_body = client.threads();
    let threads = threads_body
        .get("threads")
        .and_then(|v| v.as_array())
        .expect("threads body should have threads array");
    assert!(
        !threads.is_empty(),
        "threads should not be empty after crash halt"
    );

    // Find the thread matching the stopped event's threadId
    let crashed_thread = threads
        .iter()
        .find(|t| t.get("id").and_then(|v| v.as_i64()) == Some(thread_id));
    assert!(
        crashed_thread.is_some(),
        "crashed thread_id {} should appear in threads list: {:?}",
        thread_id,
        threads
    );

    let thread_name = crashed_thread.unwrap()
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_ne!(
        thread_name, "terminated",
        "crashed thread name should not be 'terminated', got: '{}'",
        thread_name
    );

    // StackTrace — should return non-empty live frames (break-before-unwind, primary path)
    let stack_body = client.stack_trace(thread_id);
    let frames = stack_body
        .get("stackFrames")
        .and_then(|v| v.as_array())
        .expect("stackTrace should have stackFrames array");
    assert!(
        !frames.is_empty(),
        "stackFrames should not be empty after crash halt (live frames via break-before-unwind)"
    );

    // --- Variable inspection during crash halt ---
    // Extract frame_id from the top stack frame
    let frame_id = frames[0]
        .get("id")
        .and_then(|v| v.as_i64())
        .expect("top crash frame should have id");

    // Scopes — get variablesReference for the crash frame
    let scopes_body = client.scopes(frame_id);
    let scopes = scopes_body
        .get("scopes")
        .and_then(|v| v.as_array())
        .expect("scopes should have scopes array for crash frame");
    assert!(
        !scopes.is_empty(),
        "scopes should not be empty for crash frame"
    );

    let vars_ref = scopes[0]
        .get("variablesReference")
        .and_then(|v| v.as_i64())
        .expect("crash scope should have variablesReference");
    assert_ne!(
        vars_ref, 0,
        "variablesReference must be non-zero for crash frame (0 means no children), got: {}",
        vars_ref
    );

    // Variables — fetch locals using the dynamic variablesReference
    let vars_body = client.variables(vars_ref);
    let variables = vars_body
        .get("variables")
        .and_then(|v| v.as_array())
        .expect("crash frame variables should have variables array");
    assert!(
        !variables.is_empty(),
        "variables should not be empty during crash halt -- locals should be preserved from crash point"
    );

    // Verify the "x" variable is present (let x: int? = None)
    let x_var = variables
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("x"));
    assert!(
        x_var.is_some(),
        "variable 'x' should be visible during crash halt, got variables: {:?}",
        variables.iter().map(|v| v.get("name")).collect::<Vec<_>>()
    );

    // Continue after crash: task is Suspended+CrashPending; resume_debug() performs the
    // deferred unwind (task becomes Cancelled), then terminated+exited(1) is emitted.
    let (_cont_resp, cont_events) = client.continue_(thread_id);
    let has_terminated = cont_events
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("terminated"));
    assert!(
        has_terminated,
        "continue after crash should emit terminated event, got: {:?}",
        cont_events
            .iter()
            .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
            .collect::<Vec<_>>()
    );

    // Verify exit code is 1 (crash)
    let exited = cont_events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("exited"));
    if let Some(exited_event) = exited {
        let exit_code = exited_event
            .get("body")
            .and_then(|b| b.get("exitCode"))
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        assert_eq!(
            exit_code, 1,
            "exit code should be 1 for a crash, got: {}",
            exit_code
        );
    }

    client.shutdown();
}

/// Test: Variable names are present and correct on breakpoint hit at line 12.
///
/// Validates that the DAP variables response:
/// - Contains no unnamed (empty-string) entries — unnamed temporaries must be filtered
/// - Contains a variable named "x" with value "7" and type "int"
///
/// Breaking on line 12 (`let flag: bool = is_positive(x);`) ensures `x` is already
/// assigned (line 11 completed), so it is live and inspectable.
#[test]
fn test_breakpoint_variables_have_names() {
    let fixture_path = workspace_file(FIXTURE);
    let mut client = DapClient::start();

    client.initialize();

    // Set breakpoint on line 12: `let flag: bool = is_positive(x);`
    // At this point x = add(3, 4) = 7 is in scope.
    let bp_body = client.set_breakpoints(&fixture_path, &[12]);
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(|v| v.as_array())
        .expect("should have breakpoints array");
    assert_eq!(breakpoints.len(), 1, "should have 1 breakpoint entry");

    client.configuration_done();

    // Launch — should stop at breakpoint
    let (launch_resp, events) = client.launch(&fixture_path, false);
    assert_eq!(
        launch_resp.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "launch should succeed, got: {}",
        launch_resp
    );

    // Extract stopped event
    let stopped = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"))
        .expect("should receive stopped event at breakpoint");

    let stopped_body = stopped.get("body").cloned().unwrap_or(json!({}));
    assert_eq!(
        stopped_body.get("reason").and_then(|v| v.as_str()),
        Some("breakpoint"),
        "stopped reason should be 'breakpoint', got: {:?}",
        stopped_body.get("reason")
    );

    // Extract dynamic threadId
    let thread_id = stopped_body
        .get("threadId")
        .and_then(|v| v.as_i64())
        .expect("stopped event should have threadId");

    // StackTrace → extract frameId
    let stack_body = client.stack_trace(thread_id);
    let frames = stack_body
        .get("stackFrames")
        .and_then(|v| v.as_array())
        .expect("stackTrace should have stackFrames array");
    assert!(!frames.is_empty(), "should have at least 1 stack frame");

    let frame_id = frames[0]
        .get("id")
        .and_then(|v| v.as_i64())
        .expect("top frame should have id");

    // Scopes → extract variablesReference
    let scopes_body = client.scopes(frame_id);
    let scopes = scopes_body
        .get("scopes")
        .and_then(|v| v.as_array())
        .expect("scopes should have scopes array");
    assert!(!scopes.is_empty(), "should have at least 1 scope");

    let vars_ref = scopes[0]
        .get("variablesReference")
        .and_then(|v| v.as_i64())
        .expect("scope should have variablesReference");
    assert_ne!(vars_ref, 0, "variablesReference must be non-zero");

    // Variables — fetch and inspect
    let vars_body = client.variables(vars_ref);
    let variables = vars_body
        .get("variables")
        .and_then(|v| v.as_array())
        .expect("variables should have variables array");

    assert!(
        !variables.is_empty(),
        "should have at least 1 variable in scope at line 12 (x should be live)"
    );

    // Assert: no variable has an empty name (unnamed temporaries must be filtered out)
    let variable_names: Vec<&str> = variables
        .iter()
        .map(|v| v.get("name").and_then(|n| n.as_str()).unwrap_or(""))
        .collect();
    for name in &variable_names {
        assert!(
            !name.is_empty(),
            "all variables must have non-empty names, found empty name in: {:?}",
            variable_names
        );
    }

    // Assert: variable "x" is present with value "7" and type "int"
    let x_var = variables
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("x"))
        .expect(&format!(
            "variable 'x' should be in scope at line 12, got variables: {:?}",
            variable_names
        ));

    let x_value = x_var.get("value").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(
        x_value, "7",
        "x should have value '7' (result of add(3,4)), got: '{}'",
        x_value
    );

    let x_type = x_var
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(
        x_type, "int",
        "x should have type 'int', got: '{}'",
        x_type
    );

    // Continue → should terminate
    let (_cont_resp, cont_events) = client.continue_(thread_id);
    let has_terminated = cont_events
        .iter()
        .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("terminated"));
    assert!(
        has_terminated,
        "should receive terminated event after continue, events: {:?}",
        cont_events
            .iter()
            .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
            .collect::<Vec<_>>()
    );

    client.shutdown();
}

/// Test 6: Unsupported command returns an error response.
#[test]
fn test_unknown_command_returns_error() {
    let mut client = DapClient::start();

    client.initialize();

    let seq = client.send("restart", json!({}));
    let (resp, _events) = client.recv_response(seq);

    assert_eq!(
        resp.get("success").and_then(|v| v.as_bool()),
        Some(false),
        "unsupported command should return success=false, got: {}",
        resp
    );

    client.shutdown();
}
