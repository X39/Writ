//! Register allocator for IL method body emission.
//!
//! Assigns sequential u16 register indices. Parameters occupy r0..rN-1,
//! locals and temporaries get rN+. reg_count() returns the peak (next unallocated).

use crate::check::ty::Ty;

/// A simple sequential register allocator.
///
/// No spilling — this is a high-watermark allocator. Register indices are
/// monotonically increasing: each call to `alloc` returns the next available slot.
pub struct RegisterAllocator {
    next: u16,
    types: Vec<Ty>,
}

impl RegisterAllocator {
    /// Create a new empty allocator.
    pub fn new() -> Self {
        Self {
            next: 0,
            types: Vec::new(),
        }
    }

    /// Allocate a new register with the given type. Returns its index.
    pub fn alloc(&mut self, ty: Ty) -> u16 {
        let idx = self.next;
        self.next += 1;
        self.types.push(ty);
        idx
    }

    /// Return the total number of registers allocated (= next unallocated index).
    pub fn reg_count(&self) -> u16 {
        self.next
    }

    /// Return the per-register type table (parallel to register indices).
    pub fn types(&self) -> &[Ty] {
        &self.types
    }

    /// Return the next register index without allocating it.
    ///
    /// Used for argument packing: to know where the consecutive block would start.
    pub fn next(&self) -> u16 {
        self.next
    }

    /// Return the type of an already-allocated register.
    ///
    /// Panics if the register index is out of range.
    pub fn type_of(&self, reg: u16) -> Ty {
        self.types[reg as usize]
    }
}

impl Default for RegisterAllocator {
    fn default() -> Self {
        Self::new()
    }
}
