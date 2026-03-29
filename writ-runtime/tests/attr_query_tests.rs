//! Integration tests for ModuleAttributeView and the on_module_load pre-load callback.
//!
//! These tests verify QAPI-04 (pre-load callback), QAPI-05 (rejection), and
//! QAPI-06 (no side effects from inspection).

use std::sync::{Arc, Mutex};

use writ_module::ModuleBuilder;
use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL, TypeDefKind};
use writ_module::token::MetadataToken;
use writ_module::attr::{encode_attr_args, AttrValue};

use writ_runtime::{
    RuntimeBuilder, RuntimeHost, HostRequest, HostResponse, RequestId, LogLevel,
    ModuleAttributeView,
};

// ── Helper: build a test module with attributes ──────────────────────────────

fn make_test_module() -> writ_module::Module {
    let mut b = ModuleBuilder::new("QuestModule");
    // TypeDef "QuestGiver" at index 0 (row 1)
    b.add_type_def("QuestGiver", "", TypeDefKind::Struct, 0);
    let typedef_token = MetadataToken::new(TableId::TypeDef.as_u8(), 1);
    // Declaration row: should be filtered by query_attributes
    b.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, "Quest", &[]);
    // Application row on QuestGiver typedef
    let encoded = encode_attr_args(&[AttrValue::String("Chapter1".into())]);
    b.add_attribute_def(typedef_token, 0, "Quest", &encoded);
    b.build()
}

// ── Custom test host using Arc<Mutex<>> for shared state ─────────────────────

struct RecordingHost {
    data: Arc<Mutex<HostData>>,
    reject: Option<String>,
}

#[derive(Default)]
struct HostData {
    load_call_count: usize,
    module_name: Option<String>,
    quest_matches: Vec<String>, // attribute names recorded
    typedef0_matches: Vec<String>,
}

impl RecordingHost {
    fn new(reject: Option<String>) -> (Self, Arc<Mutex<HostData>>) {
        let data = Arc::new(Mutex::new(HostData::default()));
        let host = RecordingHost {
            data: Arc::clone(&data),
            reject,
        };
        (host, data)
    }
}

impl RuntimeHost for RecordingHost {
    fn on_request(&mut self, _id: RequestId, _req: &HostRequest) -> HostResponse {
        HostResponse::Value(writ_runtime::Value::Void)
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}

    fn on_module_load(&mut self, view: &ModuleAttributeView<'_>) -> Result<(), String> {
        let mut guard = self.data.lock().unwrap();
        guard.load_call_count += 1;
        guard.module_name = Some(view.module_name().to_owned());

        // Record Quest attribute matches
        for m in view.query_attributes("Quest") {
            guard.quest_matches.push(m.name.clone());
        }

        // Record attributes on typedef index 0
        for m in view.query_attributes_on(0) {
            guard.typedef0_matches.push(m.name.clone());
        }

        if let Some(ref reason) = self.reject {
            Err(reason.clone())
        } else {
            Ok(())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn on_module_load_fires_for_user_module() {
    let module = make_test_module();
    let (host, data) = RecordingHost::new(None);
    let _rt = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("build should succeed");

    let guard = data.lock().unwrap();
    assert_eq!(guard.load_call_count, 1, "on_module_load should fire exactly once");
}

#[test]
fn on_module_load_view_has_correct_module_name() {
    let module = make_test_module();
    let (host, data) = RecordingHost::new(None);
    let _rt = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("build should succeed");

    let guard = data.lock().unwrap();
    assert_eq!(
        guard.module_name.as_deref(),
        Some("QuestModule"),
        "module_name should match the module built with ModuleBuilder::new(\"QuestModule\")"
    );
}

#[test]
fn on_module_load_rejection_prevents_loading() {
    let module = make_test_module();
    let (host, _data) = RecordingHost::new(Some("bad module".to_owned()));
    let result = RuntimeBuilder::new(module)
        .with_host(host)
        .build();

    assert!(result.is_err(), "build should fail when host rejects module");
    let err_str = format!("{}", result.err().unwrap());
    assert!(
        err_str.contains("module rejected by host"),
        "error message should contain 'module rejected by host', got: {err_str}"
    );
    assert!(
        err_str.contains("bad module"),
        "error message should contain the rejection reason, got: {err_str}"
    );
}

#[test]
fn on_module_load_allows_attribute_query() {
    let module = make_test_module();
    let (host, data) = RecordingHost::new(None);
    let _rt = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("build should succeed");

    let guard = data.lock().unwrap();
    // Should have exactly 1 match (application row), not 2 (declaration row filtered)
    assert_eq!(
        guard.quest_matches.len(), 1,
        "should find 1 Quest attribute application (declaration filtered), got {:?}",
        guard.quest_matches
    );
    assert_eq!(guard.quest_matches[0], "Quest");
}

#[test]
fn on_module_load_query_attributes_on_typedef() {
    let module = make_test_module();
    let (host, data) = RecordingHost::new(None);
    let _rt = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("build should succeed");

    let guard = data.lock().unwrap();
    assert_eq!(
        guard.typedef0_matches.len(), 1,
        "should find 1 attribute on typedef[0], got {:?}",
        guard.typedef0_matches
    );
    assert_eq!(guard.typedef0_matches[0], "Quest");
}

// ── Domain query tests ────────────────────────────────────────────────────────

fn build_runtime_with_quest_attr() -> writ_runtime::Runtime<impl writ_runtime::RuntimeHost> {
    use writ_runtime::{RuntimeBuilder, NullHost};

    let mut b = ModuleBuilder::new("test-module");
    b.add_type_def("QuestGiver", "", TypeDefKind::Struct, 0);
    let typedef_token = MetadataToken::new(TableId::TypeDef.as_u8(), 1);
    // Declaration row — must be filtered out by query methods
    b.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, "Quest", &[]);
    // Application row — owner_kind=0 (type application)
    let encoded = encode_attr_args(&[AttrValue::String("Chapter1".into())]);
    b.add_attribute_def(typedef_token, 0, "Quest", &encoded);
    let module = b.build();
    RuntimeBuilder::new(module)
        .with_host(NullHost)
        .build()
        .expect("RuntimeBuilder::build should succeed")
}

#[test]
fn domain_query_attributes_by_name() {
    let rt = build_runtime_with_quest_attr();
    let matches = rt.domain().query_attributes("Quest");
    assert_eq!(matches.len(), 1, "expected 1 Quest match, got {}", matches.len());
    assert_eq!(matches[0].name, "Quest");
    assert_eq!(matches[0].args, vec![AttrValue::String("Chapter1".into())]);
}

#[test]
fn domain_query_attributes_excludes_declarations() {
    let rt = build_runtime_with_quest_attr();
    // Module has 1 application + 1 declaration; only 1 should be returned
    let matches = rt.domain().query_attributes("Quest");
    assert_eq!(matches.len(), 1, "declaration rows must be excluded; expected 1, got {}", matches.len());
}

#[test]
fn domain_query_attributes_no_match() {
    let rt = build_runtime_with_quest_attr();
    let matches = rt.domain().query_attributes("Missing");
    assert!(matches.is_empty(), "expected empty vec for non-existent attribute");
}

#[test]
fn domain_query_attributes_on_typedef() {
    let rt = build_runtime_with_quest_attr();
    let matches = rt.domain().query_attributes_on(rt.user_module_idx(), 0);
    assert_eq!(matches.len(), 1, "expected 1 attribute on typedef[0], got {}", matches.len());
    assert_eq!(matches[0].name, "Quest");
}

#[test]
fn domain_query_attributes_on_wrong_typedef() {
    let rt = build_runtime_with_quest_attr();
    let matches = rt.domain().query_attributes_on(rt.user_module_idx(), 99);
    assert!(matches.is_empty(), "expected empty vec for non-existent typedef index");
}

#[test]
fn domain_query_attribute_value_found() {
    let rt = build_runtime_with_quest_attr();
    let typedef_token = MetadataToken::new(TableId::TypeDef.as_u8(), 1);
    let args = rt.domain().query_attribute_value(rt.user_module_idx(), typedef_token, "Quest");
    assert!(args.is_some(), "expected Some for existing attribute");
    assert_eq!(args.unwrap(), vec![AttrValue::String("Chapter1".into())]);
}

#[test]
fn domain_query_attribute_value_not_found() {
    let rt = build_runtime_with_quest_attr();
    let typedef_token = MetadataToken::new(TableId::TypeDef.as_u8(), 1);
    let args = rt.domain().query_attribute_value(rt.user_module_idx(), typedef_token, "Missing");
    assert!(args.is_none(), "expected None for non-existent attribute name");
}
