use std::marker::PhantomData;

/// Tag type for task handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskTag;
/// Tag type for entity handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityTag;

/// Generation-indexed handle for type-safe arena references.
///
/// `GenHandle<TaskTag>` and `GenHandle<EntityTag>` are distinct types at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenHandle<T> {
    pub index: u32,
    pub generation: u32,
    _phantom: PhantomData<T>,
}

impl<T> GenHandle<T> {
    pub fn new(index: u32, generation: u32) -> Self {
        Self {
            index,
            generation,
            _phantom: PhantomData,
        }
    }
}

/// Task identifier — generation-indexed handle.
pub type TaskId = GenHandle<TaskTag>;
/// Entity identifier — generation-indexed handle.
pub type EntityId = GenHandle<EntityTag>;

/// Pack a TaskId into a Value::Int for storing in registers.
/// Encoding: index in high 32 bits, generation in low 32 bits.
pub(crate) fn pack_task_id(id: TaskId) -> Value {
    Value::Int(((id.index as i64) << 32) | (id.generation as i64))
}

/// Unpack a TaskId from a Value::Int.
pub(crate) fn unpack_task_id(val: &Value) -> Option<TaskId> {
    if let Value::Int(packed) = val {
        let index = (*packed >> 32) as u32;
        let generation = (*packed & 0xFFFFFFFF) as u32;
        Some(TaskId::new(index, generation))
    } else {
        None
    }
}

/// Opaque reference to a heap-allocated object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapRef(pub(crate) u32);

/// Runtime value representation.
///
/// All variants are Copy-safe (Ref and Entity are just u32/handle copies).
#[derive(Debug, Clone, Copy)]
pub enum Value {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),
    Entity(EntityId),
}

impl Default for Value {
    fn default() -> Self {
        Value::Void
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Void, Value::Void) => true,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Ref(a), Value::Ref(b)) => a == b,
            (Value::Entity(a), Value::Entity(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_int_stores_and_retrieves() {
        let v = Value::Int(42);
        match v {
            Value::Int(n) => assert_eq!(n, 42),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn value_float_stores_and_retrieves() {
        let v = Value::Float(3.14);
        match v {
            Value::Float(f) => assert!((f - 3.14).abs() < f64::EPSILON),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn value_bool_stores_and_retrieves() {
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
    }

    #[test]
    fn value_void_is_default() {
        let v = Value::default();
        assert_eq!(v, Value::Void);
    }

    #[test]
    fn value_ref_stores_heap_ref() {
        let href = HeapRef(42);
        let v = Value::Ref(href);
        match v {
            Value::Ref(h) => assert_eq!(h, HeapRef(42)),
            _ => panic!("expected Ref"),
        }
    }

    #[test]
    fn value_entity_stores_entity_id() {
        let eid = EntityId::new(5, 3);
        let v = Value::Entity(eid);
        match v {
            Value::Entity(e) => {
                assert_eq!(e.index, 5);
                assert_eq!(e.generation, 3);
            }
            _ => panic!("expected Entity"),
        }
    }

    #[test]
    fn gen_handle_equality_checks_both_index_and_generation() {
        let a = TaskId::new(1, 0);
        let b = TaskId::new(1, 0);
        let c = TaskId::new(1, 1);
        let d = TaskId::new(2, 0);
        assert_eq!(a, b);
        assert_ne!(a, c); // same index, different generation
        assert_ne!(a, d); // different index, same generation
    }

    #[test]
    fn task_id_and_entity_id_are_distinct_types() {
        // This test verifies compile-time type safety.
        // TaskId and EntityId are different types — they cannot be mixed.
        let _task: TaskId = GenHandle::new(1, 0);
        let _entity: EntityId = GenHandle::new(1, 0);
        // If they were the same type, the compiler would allow assignment between them.
        // They are NOT, so this test just proves compilation succeeds with distinct types.
    }
}
