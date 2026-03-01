use crate::error::RuntimeError;
use crate::heap::HeapObject;
use crate::value::{HeapRef, Value};

/// Statistics from a garbage collection cycle.
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    pub objects_traced: usize,
    pub objects_freed: usize,
    pub heap_before: usize,
    pub heap_after: usize,
    pub finalization_queue_size: usize,
}

/// GC trigger mode (host-configurable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcMode {
    /// Host explicitly triggers collection.
    Manual,
}

/// Abstraction over heap implementations, allowing GC to be swapped without
/// changing the VM dispatch loop (GC-05).
pub trait GcHeap {
    // ── Allocation ────────────────────────────────────────────
    fn alloc_string(&mut self, s: &str) -> HeapRef;
    fn alloc_struct(&mut self, field_count: usize) -> HeapRef;
    fn alloc_array(&mut self, elem_type: u32) -> HeapRef;
    fn alloc_delegate(&mut self, method_idx: usize, target: Option<Value>) -> HeapRef;
    fn alloc_enum(&mut self, type_idx: u32, tag: u16, fields: Vec<Value>) -> HeapRef;
    fn alloc_boxed(&mut self, val: Value) -> HeapRef;

    // ── Access ────────────────────────────────────────────────
    fn read_string(&self, href: HeapRef) -> Result<&str, RuntimeError>;
    fn get_field(&self, href: HeapRef, idx: usize) -> Result<Value, RuntimeError>;
    fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError>;
    fn get_object(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError>;
    fn get_object_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError>;

    // ── Collection ────────────────────────────────────────────
    fn collect(&mut self, roots: &[HeapRef]) -> GcStats;

    // ── Finalization ──────────────────────────────────────────
    /// Mark an object as having a finalizer (survives one extra GC cycle).
    fn mark_finalizable(&mut self, _href: HeapRef) {}

    /// Drain and return the finalization queue (objects awaiting on_finalize).
    fn drain_finalization_queue(&mut self) -> Vec<HeapRef> {
        Vec::new()
    }

    // ── Inspection ────────────────────────────────────────────
    fn heap_size(&self) -> usize;
    fn object_count(&self) -> usize;
}

/// Extract all HeapRef values reachable from a HeapObject (one level deep).
pub fn trace_refs(obj: &HeapObject) -> Vec<HeapRef> {
    let mut refs = Vec::new();
    match obj {
        HeapObject::String(_) => {}
        HeapObject::Struct { fields, .. } => {
            for v in fields {
                if let Value::Ref(href) = v {
                    refs.push(*href);
                }
            }
        }
        HeapObject::Array { elements, .. } => {
            for v in elements {
                if let Value::Ref(href) = v {
                    refs.push(*href);
                }
            }
        }
        HeapObject::Delegate { target, .. } => {
            if let Some(Value::Ref(href)) = target {
                refs.push(*href);
            }
        }
        HeapObject::Enum { fields, .. } => {
            for v in fields {
                if let Value::Ref(href) = v {
                    refs.push(*href);
                }
            }
        }
        HeapObject::Boxed(v) => {
            if let Value::Ref(href) = v {
                refs.push(*href);
            }
        }
    }
    refs
}

/// Mark-and-sweep garbage collector with finalization support.
pub struct MarkSweepHeap {
    objects: Vec<Option<HeapObject>>,
    marks: Vec<bool>,
    free_list: Vec<u32>,
    has_finalizer: Vec<bool>,
    finalization_queue: Vec<HeapRef>,
}

impl MarkSweepHeap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            marks: Vec::new(),
            free_list: Vec::new(),
            has_finalizer: Vec::new(),
            finalization_queue: Vec::new(),
        }
    }

    fn alloc_slot(&mut self, obj: HeapObject) -> HeapRef {
        if let Some(idx) = self.free_list.pop() {
            let i = idx as usize;
            self.objects[i] = Some(obj);
            self.marks[i] = false;
            self.has_finalizer[i] = false;
            HeapRef(idx)
        } else {
            let idx = self.objects.len() as u32;
            self.objects.push(Some(obj));
            self.marks.push(false);
            self.has_finalizer.push(false);
            HeapRef(idx)
        }
    }

    fn get_obj(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError> {
        match self.objects.get(href.0 as usize) {
            Some(Some(obj)) => Ok(obj),
            Some(None) => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} has been freed",
                href.0
            ))),
            None => Err(RuntimeError::ExecutionError(format!(
                "invalid heap reference: {}",
                href.0
            ))),
        }
    }

    fn get_obj_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError> {
        match self.objects.get_mut(href.0 as usize) {
            Some(Some(obj)) => Ok(obj),
            Some(None) => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} has been freed",
                href.0
            ))),
            None => Err(RuntimeError::ExecutionError(format!(
                "invalid heap reference: {}",
                href.0
            ))),
        }
    }
}

impl Default for MarkSweepHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl GcHeap for MarkSweepHeap {
    fn alloc_string(&mut self, s: &str) -> HeapRef {
        self.alloc_slot(HeapObject::String(s.to_string()))
    }

    fn alloc_struct(&mut self, field_count: usize) -> HeapRef {
        self.alloc_slot(HeapObject::Struct {
            fields: vec![Value::Void; field_count],
        })
    }

    fn alloc_array(&mut self, elem_type: u32) -> HeapRef {
        self.alloc_slot(HeapObject::Array {
            elem_type,
            elements: Vec::new(),
        })
    }

    fn alloc_delegate(&mut self, method_idx: usize, target: Option<Value>) -> HeapRef {
        self.alloc_slot(HeapObject::Delegate { method_idx, target })
    }

    fn alloc_enum(&mut self, type_idx: u32, tag: u16, fields: Vec<Value>) -> HeapRef {
        self.alloc_slot(HeapObject::Enum {
            type_idx,
            tag,
            fields,
        })
    }

    fn alloc_boxed(&mut self, val: Value) -> HeapRef {
        self.alloc_slot(HeapObject::Boxed(val))
    }

    fn read_string(&self, href: HeapRef) -> Result<&str, RuntimeError> {
        match self.get_obj(href)? {
            HeapObject::String(s) => Ok(s.as_str()),
            _ => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} is not a string",
                href.0
            ))),
        }
    }

    fn get_field(&self, href: HeapRef, idx: usize) -> Result<Value, RuntimeError> {
        match self.get_obj(href)? {
            HeapObject::Struct { fields, .. } => fields.get(idx).copied().ok_or_else(|| {
                RuntimeError::ExecutionError(format!(
                    "field index {} out of range for struct with {} fields",
                    idx,
                    fields.len()
                ))
            }),
            HeapObject::Enum { fields, .. } => fields.get(idx).copied().ok_or_else(|| {
                RuntimeError::ExecutionError(format!(
                    "field index {} out of range for enum with {} fields",
                    idx,
                    fields.len()
                ))
            }),
            _ => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} does not have fields",
                href.0
            ))),
        }
    }

    fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError> {
        match self.get_obj_mut(href)? {
            HeapObject::Struct { fields, .. } => {
                if idx < fields.len() {
                    fields[idx] = val;
                    Ok(())
                } else {
                    Err(RuntimeError::ExecutionError(format!(
                        "field index {} out of range for struct with {} fields",
                        idx,
                        fields.len()
                    )))
                }
            }
            _ => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} is not a struct",
                href.0
            ))),
        }
    }

    fn get_object(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError> {
        self.get_obj(href)
    }

    fn get_object_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError> {
        self.get_obj_mut(href)
    }

    fn collect(&mut self, roots: &[HeapRef]) -> GcStats {
        let heap_before = self.heap_size();

        // Clear marks
        for m in self.marks.iter_mut() {
            *m = false;
        }

        // Mark phase 1: trace from roots and existing finalization queue
        let mut work_stack: Vec<HeapRef> = Vec::new();

        for root in roots {
            work_stack.push(*root);
        }

        // Existing finalization queue items are roots (their references must stay alive)
        for href in &self.finalization_queue {
            work_stack.push(*href);
        }

        let mut objects_traced = 0;

        while let Some(href) = work_stack.pop() {
            let idx = href.0 as usize;
            if idx >= self.objects.len() {
                continue;
            }
            if self.marks[idx] {
                continue;
            }
            if self.objects[idx].is_none() {
                continue;
            }

            self.marks[idx] = true;
            objects_traced += 1;

            if let Some(ref obj) = self.objects[idx] {
                let refs = trace_refs(obj);
                for child_ref in refs {
                    if (child_ref.0 as usize) < self.objects.len()
                        && !self.marks[child_ref.0 as usize]
                    {
                        work_stack.push(child_ref);
                    }
                }
            }
        }

        // Pre-sweep: identify unmarked finalizable objects and move them to the
        // finalization queue. Then re-mark them and their transitive references
        // so that nothing reachable from a finalizable object is freed this cycle
        // (topological finalization — GC-04).
        let mut newly_queued: Vec<HeapRef> = Vec::new();
        for i in 0..self.objects.len() {
            if !self.marks[i] && self.objects[i].is_some() && self.has_finalizer[i] {
                self.finalization_queue.push(HeapRef(i as u32));
                self.has_finalizer[i] = false; // Clear so it can be freed next cycle
                newly_queued.push(HeapRef(i as u32));
            }
        }

        // Re-mark newly queued finalizable objects and everything they reference
        for href in newly_queued {
            work_stack.push(href);
        }
        while let Some(href) = work_stack.pop() {
            let idx = href.0 as usize;
            if idx >= self.objects.len() || self.marks[idx] || self.objects[idx].is_none() {
                continue;
            }
            self.marks[idx] = true;
            objects_traced += 1;
            if let Some(ref obj) = self.objects[idx] {
                let refs = trace_refs(obj);
                for child_ref in refs {
                    if (child_ref.0 as usize) < self.objects.len()
                        && !self.marks[child_ref.0 as usize]
                    {
                        work_stack.push(child_ref);
                    }
                }
            }
        }

        // Sweep phase: free all unmarked, non-finalizable objects
        let mut objects_freed = 0;
        for i in 0..self.objects.len() {
            if !self.marks[i] && self.objects[i].is_some() {
                self.objects[i] = None;
                self.free_list.push(i as u32);
                objects_freed += 1;
            }
        }

        let heap_after = self.heap_size();

        GcStats {
            objects_traced,
            objects_freed,
            heap_before,
            heap_after,
            finalization_queue_size: self.finalization_queue.len(),
        }
    }

    fn mark_finalizable(&mut self, href: HeapRef) {
        let idx = href.0 as usize;
        if idx < self.has_finalizer.len() {
            self.has_finalizer[idx] = true;
        }
    }

    fn drain_finalization_queue(&mut self) -> Vec<HeapRef> {
        std::mem::take(&mut self.finalization_queue)
    }

    fn heap_size(&self) -> usize {
        self.objects.iter().filter(|o| o.is_some()).count()
    }

    fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BumpHeap as GcHeap tests ──────────────────────────────

    #[test]
    fn bump_heap_implements_gc_heap() {
        use crate::heap::BumpHeap;
        let mut heap: Box<dyn GcHeap> = Box::new(BumpHeap::new());
        let href = heap.alloc_string("hello");
        assert_eq!(heap.read_string(href).unwrap(), "hello");
    }

    #[test]
    fn bump_heap_collect_is_noop() {
        use crate::heap::BumpHeap;
        let mut heap = BumpHeap::new();
        let stats = GcHeap::collect(&mut heap, &[]);
        assert_eq!(stats.objects_traced, 0);
        assert_eq!(stats.objects_freed, 0);
    }

    #[test]
    fn bump_heap_size_and_count() {
        use crate::heap::BumpHeap;
        let mut heap = BumpHeap::new();
        assert_eq!(GcHeap::heap_size(&heap), 0);
        assert_eq!(GcHeap::object_count(&heap), 0);
        GcHeap::alloc_string(&mut heap, "hello");
        GcHeap::alloc_struct(&mut heap, 3);
        assert_eq!(GcHeap::heap_size(&heap), 2);
        assert_eq!(GcHeap::object_count(&heap), 2);
    }

    // ── MarkSweepHeap tests ───────────────────────────────────

    #[test]
    fn ms_alloc_and_read_string() {
        let mut heap = MarkSweepHeap::new();
        let href = heap.alloc_string("hello");
        assert_eq!(heap.read_string(href).unwrap(), "hello");
    }

    #[test]
    fn ms_alloc_struct_and_fields() {
        let mut heap = MarkSweepHeap::new();
        let href = GcHeap::alloc_struct(&mut heap, 3);
        assert_eq!(GcHeap::get_field(&heap, href, 0).unwrap(), Value::Void);
        GcHeap::set_field(&mut heap, href, 1, Value::Int(42)).unwrap();
        assert_eq!(GcHeap::get_field(&heap, href, 1).unwrap(), Value::Int(42));
    }

    #[test]
    fn ms_collect_frees_unreachable() {
        let mut heap = MarkSweepHeap::new();
        let _unreachable = GcHeap::alloc_string(&mut heap, "gone");
        let reachable = GcHeap::alloc_string(&mut heap, "kept");
        assert_eq!(heap.heap_size(), 2);

        let stats = heap.collect(&[reachable]);
        assert_eq!(stats.objects_freed, 1);
        assert_eq!(stats.objects_traced, 1);
        assert_eq!(heap.heap_size(), 1);

        // Reachable object still accessible
        assert_eq!(heap.read_string(reachable).unwrap(), "kept");
    }

    #[test]
    fn ms_collect_traces_object_graph() {
        let mut heap = MarkSweepHeap::new();
        // A struct referencing another struct
        let child = GcHeap::alloc_string(&mut heap, "child");
        let parent = GcHeap::alloc_struct(&mut heap, 1);
        GcHeap::set_field(&mut heap, parent, 0, Value::Ref(child)).unwrap();

        // Only root parent — child should survive via tracing
        let stats = heap.collect(&[parent]);
        assert_eq!(stats.objects_traced, 2);
        assert_eq!(stats.objects_freed, 0);
        assert_eq!(heap.read_string(child).unwrap(), "child");
    }

    #[test]
    fn ms_collect_chain_a_b_c() {
        let mut heap = MarkSweepHeap::new();
        let c = GcHeap::alloc_string(&mut heap, "C");
        let b = GcHeap::alloc_struct(&mut heap, 1);
        GcHeap::set_field(&mut heap, b, 0, Value::Ref(c)).unwrap();
        let a = GcHeap::alloc_struct(&mut heap, 1);
        GcHeap::set_field(&mut heap, a, 0, Value::Ref(b)).unwrap();

        // Root only A — all three survive
        let stats = heap.collect(&[a]);
        assert_eq!(stats.objects_traced, 3);
        assert_eq!(stats.objects_freed, 0);

        // Remove root to A — all three freed
        let stats = heap.collect(&[]);
        assert_eq!(stats.objects_freed, 3);
        assert_eq!(heap.heap_size(), 0);
    }

    #[test]
    fn ms_finalizable_survives_first_collection() {
        let mut heap = MarkSweepHeap::new();
        let href = GcHeap::alloc_string(&mut heap, "finalizable");
        heap.mark_finalizable(href);

        // Collect with no roots — object should survive (in finalization queue)
        let stats = heap.collect(&[]);
        assert_eq!(stats.objects_freed, 0);
        assert_eq!(stats.finalization_queue_size, 1);
        assert_eq!(heap.heap_size(), 1); // Still alive

        // Drain finalization queue
        let queue = heap.drain_finalization_queue();
        assert_eq!(queue.len(), 1);

        // Collect again — now it should be freed
        let stats = heap.collect(&[]);
        assert_eq!(stats.objects_freed, 1);
        assert_eq!(heap.heap_size(), 0);
    }

    #[test]
    fn ms_finalization_queue_keeps_references_alive() {
        let mut heap = MarkSweepHeap::new();
        let child = GcHeap::alloc_string(&mut heap, "child");
        let parent = GcHeap::alloc_struct(&mut heap, 1);
        GcHeap::set_field(&mut heap, parent, 0, Value::Ref(child)).unwrap();
        heap.mark_finalizable(parent);

        // Collect — parent goes to finalization queue, child should stay alive
        let stats = heap.collect(&[]);
        assert_eq!(stats.objects_freed, 0); // Neither freed
        assert_eq!(stats.finalization_queue_size, 1);
        assert_eq!(heap.heap_size(), 2); // Both alive

        // Child is alive because finalization queue item keeps references alive
        assert_eq!(heap.read_string(child).unwrap(), "child");
    }

    #[test]
    fn ms_freed_slot_returns_error() {
        let mut heap = MarkSweepHeap::new();
        let href = GcHeap::alloc_string(&mut heap, "temp");
        heap.collect(&[]); // Free it
        assert!(heap.read_string(href).is_err());
    }

    #[test]
    fn ms_free_list_reuses_slots() {
        let mut heap = MarkSweepHeap::new();
        let first = GcHeap::alloc_string(&mut heap, "first");
        assert_eq!(first.0, 0);

        // Free it
        heap.collect(&[]);
        assert_eq!(heap.heap_size(), 0);

        // Allocate again — should reuse slot 0
        let reused = GcHeap::alloc_string(&mut heap, "reused");
        assert_eq!(reused.0, 0);
        assert_eq!(heap.read_string(reused).unwrap(), "reused");
    }

    #[test]
    fn ms_empty_heap_collection() {
        let mut heap = MarkSweepHeap::new();
        let stats = heap.collect(&[]);
        assert_eq!(stats.objects_traced, 0);
        assert_eq!(stats.objects_freed, 0);
        assert_eq!(stats.heap_before, 0);
        assert_eq!(stats.heap_after, 0);
    }

    #[test]
    fn ms_gc_stats_accurate() {
        let mut heap = MarkSweepHeap::new();
        let kept = GcHeap::alloc_string(&mut heap, "kept");
        let _gone1 = GcHeap::alloc_string(&mut heap, "gone1");
        let _gone2 = GcHeap::alloc_string(&mut heap, "gone2");

        let stats = heap.collect(&[kept]);
        assert_eq!(stats.heap_before, 3);
        assert_eq!(stats.objects_traced, 1);
        assert_eq!(stats.objects_freed, 2);
        assert_eq!(stats.heap_after, 1);
    }
}
