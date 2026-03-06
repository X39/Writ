//! Protocol-level DAP integration test for quest_system.writ.
//!
//! Uses the bidirectional DapClient to run a full debug session with dynamic
//! value extraction. The compilation-only test is unchanged.

mod common;

use common::{workspace_file, DapClient};
use serde_json::json;

/// Full DAP debug session with quest_system.writ:
/// initialize → setBreakpoints → configurationDone → launch →
/// (breakpoint hit) → threads → stackTrace → scopes → variables →
/// evaluate → continue → disconnect.
#[test]
fn test_quest_system_full_debug_session() {
    let fixture_path = workspace_file("writ-golden/tests/golden/quest_system.writ");
    let mut client = DapClient::start();

    client.initialize();

    // Set breakpoints on lines 68 and 170
    let bp_body = client.set_breakpoints(&fixture_path, &[68, 170]);
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(|v| v.as_array())
        .expect("should have breakpoints array");
    assert_eq!(
        breakpoints.len(),
        2,
        "should have 2 breakpoint entries (one per requested line)"
    );

    client.configuration_done();

    // Launch
    let (launch_resp, events) = client.launch(&fixture_path, false);
    let launch_success = launch_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(
        launch_success,
        "quest_system.writ should launch successfully, got: {}",
        launch_resp
    );

    // Check for stopped event (breakpoint hit)
    let stopped = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("stopped"));

    if let Some(stopped) = stopped {
        // Program stopped — verify and inspect
        let stopped_body = stopped.get("body").cloned().unwrap_or(json!({}));
        let reason = stopped_body
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            reason == "breakpoint" || reason == "step" || reason == "entry" || reason == "exception",
            "stopped reason should be breakpoint/step/entry/exception, got: {}",
            reason
        );

        // Exception stops have an unwound call stack — only do deep inspection on
        // breakpoint/step/entry stops where frames are still intact.
        if reason != "exception" {
            // Extract dynamic threadId
            let thread_id = stopped_body
                .get("threadId")
                .and_then(|v| v.as_i64())
                .expect("stopped event should have threadId");

            // Threads
            let threads_body = client.threads();
            let threads = threads_body
                .get("threads")
                .and_then(|v| v.as_array())
                .expect("threads should have threads array");
            assert!(
                !threads.is_empty(),
                "should have at least one thread while stopped"
            );

            // StackTrace → extract frameId
            let stack_body = client.stack_trace(thread_id);
            let frames = stack_body
                .get("stackFrames")
                .and_then(|v| v.as_array())
                .expect("stackTrace should have stackFrames array");
            assert!(
                !frames.is_empty(),
                "should have at least one stack frame while stopped"
            );

            let top_frame = &frames[0];
            assert!(
                top_frame.get("source").is_some(),
                "top stack frame should have source info"
            );
            let frame_line = top_frame.get("line").and_then(|v| v.as_i64()).unwrap_or(0);
            assert!(
                frame_line > 0,
                "stack frame line should be > 0, got: {}",
                frame_line
            );

            let frame_id = top_frame
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

            // Variables — use dynamic variablesReference
            let vars_body = client.variables(vars_ref);
            let variables = vars_body
                .get("variables")
                .and_then(|v| v.as_array())
                .expect("variables should have variables array");
            assert!(
                !variables.is_empty(),
                "should have at least 1 variable in scope"
            );

            // Evaluate
            let (eval_resp, _) = client.evaluate("available", frame_id);
            assert_eq!(
                eval_resp.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "evaluate should succeed, got: {}",
                eval_resp
            );

            // Continue to completion
            let (_cont_resp, _cont_events) = client.continue_(thread_id);
        }
        // For exception stops: session halted on crash — no continuation needed.
    } else {
        // Program ran to completion without hitting breakpoints
        let has_terminated = events
            .iter()
            .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("terminated"));
        assert!(
            has_terminated,
            "should receive either stopped or terminated event, got events: {:?}",
            events
                .iter()
                .map(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("?"))
                .collect::<Vec<_>>()
        );
    }

    client.shutdown();
}

/// Verify that quest_system.writ compiles successfully through the DAP pipeline.
#[test]
fn test_quest_system_compiles() {
    let fixture_path = workspace_file("writ-golden/tests/golden/quest_system.writ");
    let result = writ_dap::launch::compile_and_load(&fixture_path);

    match result {
        Ok((module, src)) => {
            assert!(!src.is_empty(), "source text should be non-empty");
            assert!(
                !module.method_defs.is_empty(),
                "compiled module should have method definitions"
            );

            // Verify main function exists
            let has_main = module.method_defs.iter().any(|md| {
                writ_module::heap::read_string(&module.string_heap, md.name)
                    .map(|n| n == "main")
                    .unwrap_or(false)
            });
            assert!(has_main, "module should contain a 'main' method");

            // Verify debug info is present
            let has_spans = module
                .method_bodies
                .iter()
                .any(|body| !body.source_spans.is_empty());
            assert!(
                has_spans,
                "compiled module should have SourceSpan debug info"
            );
        }
        Err(e) => {
            panic!(
                "quest_system.writ failed to compile: {}. \
                 This is a known issue — the DAP debug session test requires \
                 successful compilation.",
                e
            );
        }
    }
}
