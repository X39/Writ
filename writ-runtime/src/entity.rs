use std::collections::HashMap;

use crate::error::RuntimeError;
use crate::value::{EntityId, HeapRef, Value};

/// State of an entity slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    /// Slot is in the free list (not allocated).
    Free,
    /// Entity is being constructed (between SPAWN_ENTITY and INIT_ENTITY).
    Pending,
    /// Entity is alive and usable.
    Alive,
    /// Entity is in the process of being destroyed (on_destroy hook is running).
    /// When DESTROY_ENTITY re-executes after the hook, it sees this state and
    /// completes the destruction without re-firing the hook.
    Destroying,
    /// Entity has been destroyed.
    Destroyed,
}

/// A slot in the entity registry.
#[derive(Debug)]
pub struct EntitySlot {
    pub generation: u32,
    pub state: EntityState,
    pub type_idx: u32,
    pub data_ref: Option<HeapRef>,
}

/// Buffered field writes for an entity under construction.
#[derive(Debug)]
pub struct PendingEntity {
    pub type_idx: u32,
    pub field_writes: Vec<(u32, Value)>,
}

/// Manages entity lifecycle through generation-indexed handles.
///
/// Entities are allocated in a slot array. Each slot tracks a generation counter
/// that is bumped on destroy, allowing stale-handle detection. Destroyed slots
/// are recycled via a free list.
pub struct EntityRegistry {
    slots: Vec<EntitySlot>,
    free_list: Vec<u32>,
    singletons: HashMap<u32, EntityId>,
    pending: HashMap<u32, PendingEntity>, // keyed by slot index
}

impl EntityRegistry {
    /// Create an empty entity registry.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
            singletons: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    /// Allocate a new entity slot in Alive state.
    pub fn allocate(&mut self, type_idx: u32) -> EntityId {
        if let Some(idx) = self.free_list.pop() {
            let slot = &mut self.slots[idx as usize];
            slot.state = EntityState::Alive;
            slot.type_idx = type_idx;
            slot.data_ref = None;
            EntityId::new(idx, slot.generation)
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(EntitySlot {
                generation: 0,
                state: EntityState::Alive,
                type_idx,
                data_ref: None,
            });
            EntityId::new(idx, 0)
        }
    }

    /// Begin spawning a new entity (enters Pending state).
    ///
    /// The entity is not visible to the host until `commit_init` is called.
    pub fn begin_spawn(&mut self, type_idx: u32) -> EntityId {
        let entity_id = if let Some(idx) = self.free_list.pop() {
            let slot = &mut self.slots[idx as usize];
            slot.state = EntityState::Pending;
            slot.type_idx = type_idx;
            slot.data_ref = None;
            EntityId::new(idx, slot.generation)
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(EntitySlot {
                generation: 0,
                state: EntityState::Pending,
                type_idx,
                data_ref: None,
            });
            EntityId::new(idx, 0)
        };

        self.pending.insert(
            entity_id.index,
            PendingEntity {
                type_idx,
                field_writes: Vec::new(),
            },
        );

        entity_id
    }

    /// Buffer a field write for a pending entity (between SPAWN and INIT).
    pub fn buffer_field_write(
        &mut self,
        entity_id: EntityId,
        field_idx: u32,
        value: Value,
    ) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        if slot.state != EntityState::Pending {
            return Err(RuntimeError::ExecutionError(format!(
                "entity {} is not pending (state: {:?})",
                entity_id.index, slot.state
            )));
        }

        let pending = self.pending.get_mut(&entity_id.index).ok_or_else(|| {
            RuntimeError::ExecutionError(format!(
                "no pending entity for index {}",
                entity_id.index
            ))
        })?;

        pending.field_writes.push((field_idx, value));
        Ok(())
    }

    /// Commit an entity init, transitioning from Pending to Alive.
    ///
    /// Returns the buffered field writes so the caller can apply them.
    pub fn commit_init(
        &mut self,
        entity_id: EntityId,
    ) -> Result<Vec<(u32, Value)>, RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &mut self.slots[entity_id.index as usize];
        if slot.state != EntityState::Pending {
            return Err(RuntimeError::ExecutionError(format!(
                "entity {} is not pending (state: {:?})",
                entity_id.index, slot.state
            )));
        }

        slot.state = EntityState::Alive;

        let pending = self.pending.remove(&entity_id.index).ok_or_else(|| {
            RuntimeError::ExecutionError(format!(
                "no pending entity for index {}",
                entity_id.index
            ))
        })?;

        Ok(pending.field_writes)
    }

    /// Check if an entity is alive (valid handle and Alive state).
    pub fn is_alive(&self, entity_id: EntityId) -> bool {
        let idx = entity_id.index as usize;
        if idx >= self.slots.len() {
            return false;
        }
        let slot = &self.slots[idx];
        slot.generation == entity_id.generation && slot.state == EntityState::Alive
    }

    /// Get the state of an entity, returning None for stale handles.
    pub fn get_state(&self, entity_id: EntityId) -> Option<EntityState> {
        let idx = entity_id.index as usize;
        if idx >= self.slots.len() {
            return None;
        }
        let slot = &self.slots[idx];
        if slot.generation != entity_id.generation {
            return None;
        }
        Some(slot.state)
    }

    /// Begin destruction of an entity (sets state to Destroying).
    ///
    /// Used by the dispatch loop to enter the two-phase destroy protocol:
    /// 1. Set state to Destroying, fire on_destroy hook
    /// 2. When DESTROY_ENTITY re-executes, complete_destroy finalizes it
    pub fn begin_destroy(&mut self, entity_id: EntityId) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        if slot.state != EntityState::Alive {
            return Err(RuntimeError::ExecutionError(format!(
                "cannot destroy entity {} in state {:?}",
                entity_id.index, slot.state
            )));
        }
        self.slots[entity_id.index as usize].state = EntityState::Destroying;
        Ok(())
    }

    /// Complete destruction of an entity that is in the Destroying state.
    ///
    /// Bumps the generation counter and adds the slot to the free list.
    /// If the entity is a singleton, it is removed from the singleton map.
    pub fn complete_destroy(&mut self, entity_id: EntityId) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        if slot.state != EntityState::Destroying {
            return Err(RuntimeError::ExecutionError(format!(
                "cannot complete_destroy entity {} in state {:?} (expected Destroying)",
                entity_id.index, slot.state
            )));
        }

        let type_idx = slot.type_idx;

        let slot = &mut self.slots[entity_id.index as usize];
        slot.state = EntityState::Destroyed;
        slot.generation += 1;
        slot.data_ref = None;
        self.free_list.push(entity_id.index);

        // Remove from singletons if this entity was registered as one
        if let Some(&singleton_id) = self.singletons.get(&type_idx) {
            if singleton_id.index == entity_id.index
                && singleton_id.generation == entity_id.generation
            {
                self.singletons.remove(&type_idx);
            }
        }

        Ok(())
    }

    /// Destroy an entity. Returns error if handle is stale or entity is not alive.
    ///
    /// Bumps the generation counter and adds the slot to the free list.
    /// If the entity is a singleton, it is removed from the singleton map.
    pub fn destroy(&mut self, entity_id: EntityId) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        if slot.state != EntityState::Alive {
            return Err(RuntimeError::ExecutionError(format!(
                "cannot destroy entity {} in state {:?}",
                entity_id.index, slot.state
            )));
        }

        let type_idx = slot.type_idx;

        let slot = &mut self.slots[entity_id.index as usize];
        slot.state = EntityState::Destroyed;
        slot.generation += 1;
        slot.data_ref = None;
        self.free_list.push(entity_id.index);

        // Remove from singletons if this entity was registered as one
        if let Some(&singleton_id) = self.singletons.get(&type_idx) {
            if singleton_id.index == entity_id.index
                && singleton_id.generation == entity_id.generation
            {
                self.singletons.remove(&type_idx);
            }
        }

        Ok(())
    }

    /// Get the type index of an alive entity.
    pub fn get_type_idx(&self, entity_id: EntityId) -> Result<u32, RuntimeError> {
        self.validate_alive(entity_id)?;
        Ok(self.slots[entity_id.index as usize].type_idx)
    }

    /// Set the heap data reference for an entity.
    pub fn set_data_ref(
        &mut self,
        entity_id: EntityId,
        href: HeapRef,
    ) -> Result<(), RuntimeError> {
        self.validate_active(entity_id)?;
        self.slots[entity_id.index as usize].data_ref = Some(href);
        Ok(())
    }

    /// Get the heap data reference for an active entity (Pending, Alive, or Destroying).
    pub fn get_data_ref(
        &self,
        entity_id: EntityId,
    ) -> Result<Option<HeapRef>, RuntimeError> {
        self.validate_active(entity_id)?;
        Ok(self.slots[entity_id.index as usize].data_ref)
    }

    /// Register an entity as a singleton for a given type.
    pub fn register_singleton(&mut self, type_idx: u32, entity_id: EntityId) {
        self.singletons.insert(type_idx, entity_id);
    }

    /// Look up a singleton entity by type index.
    pub fn get_singleton(&self, type_idx: u32) -> Option<EntityId> {
        self.singletons.get(&type_idx).copied()
    }

    /// Iterate over all alive entity slots (for GC root collection).
    pub fn alive_entities(&self) -> impl Iterator<Item = (EntityId, &EntitySlot)> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.state == EntityState::Alive)
            .map(|(idx, slot)| {
                (EntityId::new(idx as u32, slot.generation), slot)
            })
    }

    /// Return the number of alive entities.
    pub fn alive_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.state == EntityState::Alive)
            .count()
    }

    /// Return the total number of slots (including free/destroyed).
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    // ── Internal helpers ────────────────────────────────────────

    fn validate_handle(&self, entity_id: EntityId) -> Result<(), RuntimeError> {
        let idx = entity_id.index as usize;
        if idx >= self.slots.len() {
            return Err(RuntimeError::ExecutionError(format!(
                "invalid entity index: {}",
                entity_id.index
            )));
        }
        let slot = &self.slots[idx];
        if slot.generation != entity_id.generation {
            return Err(RuntimeError::ExecutionError(format!(
                "stale entity handle: index={}, expected gen={}, got gen={}",
                entity_id.index, slot.generation, entity_id.generation
            )));
        }
        Ok(())
    }

    fn validate_alive(&self, entity_id: EntityId) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        if slot.state != EntityState::Alive {
            return Err(RuntimeError::ExecutionError(format!(
                "entity {} is not alive (state: {:?})",
                entity_id.index, slot.state
            )));
        }
        Ok(())
    }

    /// Validate that an entity handle is valid and in an active state
    /// (Pending, Alive, or Destroying — not Free or Destroyed).
    fn validate_active(&self, entity_id: EntityId) -> Result<(), RuntimeError> {
        self.validate_handle(entity_id)?;
        let slot = &self.slots[entity_id.index as usize];
        match slot.state {
            EntityState::Pending | EntityState::Alive | EntityState::Destroying => Ok(()),
            _ => Err(RuntimeError::ExecutionError(format!(
                "entity {} is not active (state: {:?})",
                entity_id.index, slot.state
            ))),
        }
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_returns_entity_id() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(42);
        assert_eq!(eid.index, 0);
        assert_eq!(eid.generation, 0);
        assert!(reg.is_alive(eid));
    }

    #[test]
    fn allocate_multiple_entities() {
        let mut reg = EntityRegistry::new();
        let a = reg.allocate(1);
        let b = reg.allocate(2);
        assert_eq!(a.index, 0);
        assert_eq!(b.index, 1);
        assert!(reg.is_alive(a));
        assert!(reg.is_alive(b));
    }

    #[test]
    fn is_alive_returns_false_for_stale_handle() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        reg.destroy(eid).unwrap();

        // Original handle is now stale (generation bumped)
        assert!(!reg.is_alive(eid));
    }

    #[test]
    fn destroy_transitions_state() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        assert_eq!(reg.get_state(eid), Some(EntityState::Alive));

        reg.destroy(eid).unwrap();

        // Original handle is stale — get_state returns None
        assert_eq!(reg.get_state(eid), None);
    }

    #[test]
    fn double_destroy_returns_error() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        reg.destroy(eid).unwrap();
        assert!(reg.destroy(eid).is_err());
    }

    #[test]
    fn destroyed_slot_recycled_via_free_list() {
        let mut reg = EntityRegistry::new();
        let first = reg.allocate(0);
        assert_eq!(first.index, 0);
        assert_eq!(first.generation, 0);

        reg.destroy(first).unwrap();

        // Allocate again — should reuse slot 0 with bumped generation
        let reused = reg.allocate(1);
        assert_eq!(reused.index, 0);
        assert_eq!(reused.generation, 1);

        // Old handle is stale
        assert!(!reg.is_alive(first));
        // New handle is alive
        assert!(reg.is_alive(reused));
    }

    #[test]
    fn get_type_idx_for_alive_entity() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(42);
        assert_eq!(reg.get_type_idx(eid).unwrap(), 42);
    }

    #[test]
    fn get_type_idx_for_stale_handle_returns_error() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(42);
        reg.destroy(eid).unwrap();
        assert!(reg.get_type_idx(eid).is_err());
    }

    #[test]
    fn set_and_get_data_ref() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        assert_eq!(reg.get_data_ref(eid).unwrap(), None);

        let href = HeapRef(5);
        reg.set_data_ref(eid, href).unwrap();
        assert_eq!(reg.get_data_ref(eid).unwrap(), Some(HeapRef(5)));
    }

    #[test]
    fn register_and_get_singleton() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(10);
        reg.register_singleton(10, eid);

        assert_eq!(reg.get_singleton(10), Some(eid));
        assert_eq!(reg.get_singleton(99), None);
    }

    #[test]
    fn destroying_singleton_removes_from_map() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(10);
        reg.register_singleton(10, eid);
        assert_eq!(reg.get_singleton(10), Some(eid));

        reg.destroy(eid).unwrap();
        assert_eq!(reg.get_singleton(10), None);
    }

    #[test]
    fn begin_spawn_creates_pending_entity() {
        let mut reg = EntityRegistry::new();
        let eid = reg.begin_spawn(5);
        assert_eq!(reg.get_state(eid), Some(EntityState::Pending));
        assert!(!reg.is_alive(eid)); // Pending is not Alive
    }

    #[test]
    fn buffer_field_write_on_pending() {
        let mut reg = EntityRegistry::new();
        let eid = reg.begin_spawn(5);
        reg.buffer_field_write(eid, 0, Value::Int(42)).unwrap();
        reg.buffer_field_write(eid, 1, Value::Bool(true)).unwrap();
    }

    #[test]
    fn buffer_field_write_on_alive_fails() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        assert!(reg.buffer_field_write(eid, 0, Value::Int(1)).is_err());
    }

    #[test]
    fn commit_init_transitions_to_alive() {
        let mut reg = EntityRegistry::new();
        let eid = reg.begin_spawn(5);

        reg.buffer_field_write(eid, 0, Value::Int(42)).unwrap();
        reg.buffer_field_write(eid, 1, Value::Bool(true)).unwrap();

        let writes = reg.commit_init(eid).unwrap();
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0], (0, Value::Int(42)));
        assert_eq!(writes[1], (1, Value::Bool(true)));

        assert!(reg.is_alive(eid));
        assert_eq!(reg.get_state(eid), Some(EntityState::Alive));
    }

    #[test]
    fn commit_init_on_alive_fails() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        assert!(reg.commit_init(eid).is_err());
    }

    #[test]
    fn alive_entities_iterator() {
        let mut reg = EntityRegistry::new();
        let a = reg.allocate(1);
        let _b = reg.allocate(2);
        let c = reg.allocate(3);

        // Destroy b
        reg.destroy(_b).unwrap();

        let alive: Vec<_> = reg.alive_entities().collect();
        assert_eq!(alive.len(), 2);
        assert_eq!(alive[0].0, a);
        assert_eq!(alive[1].0, c);
    }

    #[test]
    fn alive_count_tracks_correctly() {
        let mut reg = EntityRegistry::new();
        assert_eq!(reg.alive_count(), 0);

        let a = reg.allocate(0);
        let b = reg.allocate(0);
        assert_eq!(reg.alive_count(), 2);

        reg.destroy(a).unwrap();
        assert_eq!(reg.alive_count(), 1);

        reg.destroy(b).unwrap();
        assert_eq!(reg.alive_count(), 0);
    }

    #[test]
    fn stale_handle_after_reuse() {
        let mut reg = EntityRegistry::new();
        let first = reg.allocate(0);
        reg.destroy(first).unwrap();
        let second = reg.allocate(1);

        // First handle is stale — same index but wrong generation
        assert!(!reg.is_alive(first));
        assert!(reg.is_alive(second));
        assert!(reg.get_type_idx(first).is_err());
        assert_eq!(reg.get_type_idx(second).unwrap(), 1);
    }

    #[test]
    fn pending_entity_slot_recycled_after_init_and_destroy() {
        let mut reg = EntityRegistry::new();
        let eid = reg.begin_spawn(5);
        reg.commit_init(eid).unwrap();
        reg.destroy(eid).unwrap();

        let reused = reg.allocate(6);
        assert_eq!(reused.index, 0);
        assert_eq!(reused.generation, 1);
    }

    #[test]
    fn data_ref_cleared_on_destroy() {
        let mut reg = EntityRegistry::new();
        let eid = reg.allocate(0);
        reg.set_data_ref(eid, HeapRef(10)).unwrap();
        reg.destroy(eid).unwrap();

        // Reuse the slot
        let reused = reg.allocate(1);
        assert_eq!(reg.get_data_ref(reused).unwrap(), None);
    }
}
