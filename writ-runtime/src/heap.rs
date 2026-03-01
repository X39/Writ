use crate::error::RuntimeError;
use crate::gc::{GcHeap, GcStats};
use crate::value::{HeapRef, Value};

/// Heap-allocated object variants.
#[derive(Debug, Clone)]
pub enum HeapObject {
    String(String),
    Struct { fields: Vec<Value> },
    Array { elem_type: u32, elements: Vec<Value> },
    Delegate { method_idx: usize, target: Option<Value> },
    Enum { type_idx: u32, tag: u16, fields: Vec<Value> },
    Boxed(Value),
}

/// Simple bump allocator for heap objects.
///
/// Objects are stored in a flat `Vec<HeapObject>` indexed by `HeapRef(u32)`.
/// No garbage collection in Phase 17 — objects allocated but never freed.
pub struct BumpHeap {
    objects: Vec<HeapObject>,
}

impl BumpHeap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    /// Allocate a string on the heap.
    pub fn alloc_string(&mut self, s: &str) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::String(s.to_string()));
        HeapRef(idx)
    }

    /// Allocate a struct with `field_count` fields initialized to Void.
    pub fn alloc_struct(&mut self, field_count: usize) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::Struct {
            fields: vec![Value::Void; field_count],
        });
        HeapRef(idx)
    }

    /// Allocate an empty array with the given element type.
    pub fn alloc_array(&mut self, elem_type: u32) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::Array {
            elem_type,
            elements: Vec::new(),
        });
        HeapRef(idx)
    }

    /// Allocate a delegate (function pointer with optional captured target).
    pub fn alloc_delegate(&mut self, method_idx: usize, target: Option<Value>) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::Delegate { method_idx, target });
        HeapRef(idx)
    }

    /// Allocate an enum variant.
    pub fn alloc_enum(&mut self, type_idx: u32, tag: u16, fields: Vec<Value>) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::Enum {
            type_idx,
            tag,
            fields,
        });
        HeapRef(idx)
    }

    /// Allocate a boxed value (for generic boxing).
    pub fn alloc_boxed(&mut self, val: Value) -> HeapRef {
        let idx = self.objects.len() as u32;
        self.objects.push(HeapObject::Boxed(val));
        HeapRef(idx)
    }

    /// Read a string from a heap reference.
    pub fn read_string(&self, href: HeapRef) -> Result<&str, RuntimeError> {
        match self.objects.get(href.0 as usize) {
            Some(HeapObject::String(s)) => Ok(s.as_str()),
            Some(_) => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} is not a string",
                href.0
            ))),
            None => Err(RuntimeError::ExecutionError(format!(
                "invalid heap reference: {}",
                href.0
            ))),
        }
    }

    /// Get a field value from a struct or enum heap object.
    pub fn get_field(&self, href: HeapRef, idx: usize) -> Result<Value, RuntimeError> {
        match self.objects.get(href.0 as usize) {
            Some(HeapObject::Struct { fields }) => {
                fields.get(idx).copied().ok_or_else(|| {
                    RuntimeError::ExecutionError(format!(
                        "field index {} out of range for struct with {} fields",
                        idx,
                        fields.len()
                    ))
                })
            }
            Some(HeapObject::Enum { fields, .. }) => {
                fields.get(idx).copied().ok_or_else(|| {
                    RuntimeError::ExecutionError(format!(
                        "field index {} out of range for enum with {} fields",
                        idx,
                        fields.len()
                    ))
                })
            }
            Some(_) => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} does not have fields",
                href.0
            ))),
            None => Err(RuntimeError::ExecutionError(format!(
                "invalid heap reference: {}",
                href.0
            ))),
        }
    }

    /// Set a field value on a struct heap object.
    pub fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError> {
        match self.objects.get_mut(href.0 as usize) {
            Some(HeapObject::Struct { fields }) => {
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
            Some(_) => Err(RuntimeError::ExecutionError(format!(
                "heap object at {} is not a struct",
                href.0
            ))),
            None => Err(RuntimeError::ExecutionError(format!(
                "invalid heap reference: {}",
                href.0
            ))),
        }
    }

    /// Get a reference to a heap object.
    pub fn get_object(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError> {
        self.objects.get(href.0 as usize).ok_or_else(|| {
            RuntimeError::ExecutionError(format!("invalid heap reference: {}", href.0))
        })
    }

    /// Get a mutable reference to a heap object.
    pub fn get_object_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError> {
        self.objects.get_mut(href.0 as usize).ok_or_else(|| {
            RuntimeError::ExecutionError(format!("invalid heap reference: {}", href.0))
        })
    }
}

impl Default for BumpHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl GcHeap for BumpHeap {
    fn alloc_string(&mut self, s: &str) -> HeapRef {
        BumpHeap::alloc_string(self, s)
    }

    fn alloc_struct(&mut self, field_count: usize) -> HeapRef {
        BumpHeap::alloc_struct(self, field_count)
    }

    fn alloc_array(&mut self, elem_type: u32) -> HeapRef {
        BumpHeap::alloc_array(self, elem_type)
    }

    fn alloc_delegate(&mut self, method_idx: usize, target: Option<Value>) -> HeapRef {
        BumpHeap::alloc_delegate(self, method_idx, target)
    }

    fn alloc_enum(&mut self, type_idx: u32, tag: u16, fields: Vec<Value>) -> HeapRef {
        BumpHeap::alloc_enum(self, type_idx, tag, fields)
    }

    fn alloc_boxed(&mut self, val: Value) -> HeapRef {
        BumpHeap::alloc_boxed(self, val)
    }

    fn read_string(&self, href: HeapRef) -> Result<&str, RuntimeError> {
        BumpHeap::read_string(self, href)
    }

    fn get_field(&self, href: HeapRef, idx: usize) -> Result<Value, RuntimeError> {
        BumpHeap::get_field(self, href, idx)
    }

    fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError> {
        BumpHeap::set_field(self, href, idx, val)
    }

    fn get_object(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError> {
        BumpHeap::get_object(self, href)
    }

    fn get_object_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError> {
        BumpHeap::get_object_mut(self, href)
    }

    fn collect(&mut self, _roots: &[HeapRef]) -> GcStats {
        // BumpHeap never collects — it's a no-op adapter.
        GcStats::default()
    }

    fn heap_size(&self) -> usize {
        self.objects.len()
    }

    fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_string_and_read_back() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_string("hello");
        assert_eq!(heap.read_string(href).unwrap(), "hello");
    }

    #[test]
    fn alloc_string_multiple() {
        let mut heap = BumpHeap::new();
        let a = heap.alloc_string("foo");
        let b = heap.alloc_string("bar");
        assert_eq!(heap.read_string(a).unwrap(), "foo");
        assert_eq!(heap.read_string(b).unwrap(), "bar");
    }

    #[test]
    fn alloc_struct_and_fields() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_struct(3);

        // Initially all Void
        assert_eq!(heap.get_field(href, 0).unwrap(), Value::Void);
        assert_eq!(heap.get_field(href, 1).unwrap(), Value::Void);
        assert_eq!(heap.get_field(href, 2).unwrap(), Value::Void);

        // Set and read back
        heap.set_field(href, 1, Value::Int(42)).unwrap();
        assert_eq!(heap.get_field(href, 1).unwrap(), Value::Int(42));
    }

    #[test]
    fn struct_field_out_of_range() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_struct(2);
        assert!(heap.get_field(href, 5).is_err());
        assert!(heap.set_field(href, 5, Value::Void).is_err());
    }

    #[test]
    fn read_string_on_non_string_fails() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_struct(1);
        assert!(heap.read_string(href).is_err());
    }

    #[test]
    fn alloc_array_empty() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_array(0);
        match heap.get_object(href).unwrap() {
            HeapObject::Array { elements, .. } => assert!(elements.is_empty()),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn alloc_delegate() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_delegate(5, Some(Value::Int(10)));
        match heap.get_object(href).unwrap() {
            HeapObject::Delegate { method_idx, target } => {
                assert_eq!(*method_idx, 5);
                assert_eq!(*target, Some(Value::Int(10)));
            }
            _ => panic!("expected Delegate"),
        }
    }

    #[test]
    fn alloc_enum_variant() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_enum(0, 1, vec![Value::Int(42)]);
        match heap.get_object(href).unwrap() {
            HeapObject::Enum {
                tag, fields, ..
            } => {
                assert_eq!(*tag, 1);
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0], Value::Int(42));
            }
            _ => panic!("expected Enum"),
        }
    }

    #[test]
    fn alloc_boxed_round_trip() {
        let mut heap = BumpHeap::new();
        let href = heap.alloc_boxed(Value::Int(99));
        match heap.get_object(href).unwrap() {
            HeapObject::Boxed(v) => assert_eq!(*v, Value::Int(99)),
            _ => panic!("expected Boxed"),
        }
    }

    #[test]
    fn invalid_heap_ref() {
        let heap = BumpHeap::new();
        assert!(heap.get_object(HeapRef(999)).is_err());
        assert!(heap.read_string(HeapRef(999)).is_err());
    }
}
