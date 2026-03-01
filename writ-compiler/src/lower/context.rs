use chumsky::span::SimpleSpan;
use crate::lower::error::LoweringError;

/// A speaker scope entry for active-speaker tracking in dialogue lowering.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerScope {
    /// The speaker name (owned, converted from CST &str at lowering time).
    pub name: String,
    /// The source span where this speaker was introduced.
    pub span: SimpleSpan,
}

/// Shared mutable state threaded through every lowering pass.
///
/// Passes receive `&mut LoweringContext` and:
/// - Append errors via `emit_error()` (pipeline never halts)
/// - Push/pop speaker scopes (dialogue lowering)
/// - Track current namespace (for localization key generation)
pub struct LoweringContext {
    /// Accumulated errors — all passes append here; pipeline never halts.
    errors: Vec<LoweringError>,
    /// Stack of currently-active speakers (push on dlg entry / branch entry, pop on exit).
    speaker_stack: Vec<SpeakerScope>,
    /// Stack of namespace segments. Join with "::" to get the current namespace.
    namespace_stack: Vec<String>,
}

impl LoweringContext {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            speaker_stack: Vec::new(),
            namespace_stack: Vec::new(),
        }
    }

    /// Record a lowering error. Does NOT halt the pipeline.
    pub fn emit_error(&mut self, err: LoweringError) {
        self.errors.push(err);
    }

    /// Consume the context and return all accumulated errors.
    pub fn take_errors(self) -> Vec<LoweringError> {
        self.errors
    }

    /// Borrow accumulated errors (for inspection without consuming).
    pub fn errors(&self) -> &[LoweringError] {
        &self.errors
    }

    /// Push a new active speaker scope.
    pub fn push_speaker(&mut self, scope: SpeakerScope) {
        self.speaker_stack.push(scope);
    }

    /// Pop the most recent speaker scope.
    pub fn pop_speaker(&mut self) -> Option<SpeakerScope> {
        self.speaker_stack.pop()
    }

    /// Get the current (most recently pushed) active speaker, if any.
    pub fn current_speaker(&self) -> Option<&SpeakerScope> {
        self.speaker_stack.last()
    }

    /// Returns the current depth of the speaker stack (for save/restore at scope boundaries).
    pub fn speaker_stack_depth(&self) -> usize {
        self.speaker_stack.len()
    }

    /// Push namespace segments onto the stack (for block namespaces).
    pub fn push_namespace(&mut self, segments: &[String]) {
        self.namespace_stack.extend(segments.iter().cloned());
    }

    /// Pop `count` segments from the namespace stack.
    pub fn pop_namespace(&mut self, count: usize) {
        let new_len = self.namespace_stack.len().saturating_sub(count);
        self.namespace_stack.truncate(new_len);
    }

    /// Set the namespace stack to the given segments (for declarative namespaces).
    pub fn set_namespace(&mut self, segments: Vec<String>) {
        self.namespace_stack = segments;
    }

    /// Get the current namespace as a "::" joined string. Returns empty string if no namespace.
    pub fn current_namespace(&self) -> String {
        self.namespace_stack.join("::")
    }
}
