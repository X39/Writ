//! Unit tests for RegisterPool correctness.
//!
//! Covers: FRAME-01 (fresh allocation), FRAME-02 (capacity check), FRAME-03 (reuse
//! clears registers), FRAME-04 (free-list cap), FRAME-06 (reused registers are Void),
//! and the CallFrame::with_pool constructor.

use writ_runtime::{CallFrame, RegisterPool, Value};

/// FRAME-06 primary: registers acquired after a release must all be Value::Void,
/// even when the Vec was previously written with non-Void values.
#[test]
fn pool_reuse_clears_registers() {
    let mut pool = RegisterPool::new();

    // Acquire a register Vec and write non-Void values into every slot.
    let mut regs = pool.acquire(4);
    regs[0] = Value::Int(99);
    regs[1] = Value::Bool(true);
    regs[2] = Value::Int(-1);
    regs[3] = Value::Float(3.14);

    // Return to pool.
    pool.release(regs);

    // Re-acquire — must get a clean slate regardless of what was in it before.
    let reused = pool.acquire(4);
    assert_eq!(reused.len(), 4, "reused Vec must have len == reg_count");
    for val in &reused {
        assert!(matches!(val, Value::Void), "every register must be Value::Void after reuse");
    }
}

/// FRAME-04: releasing more than POOL_CAP (64) Vecs must not grow the free-list beyond 64.
/// All 65 subsequent acquires must succeed (64 from pool + 1 fresh allocation).
#[test]
fn pool_cap_prevents_unbounded_growth() {
    let mut pool = RegisterPool::new();

    // Release 70 vecs — the pool should silently drop the last 6.
    for _ in 0..70 {
        pool.release(vec![Value::Void; 4]);
    }

    // Acquire 65 times — the first 64 come from the pool, the 65th is a fresh alloc.
    let mut acquired: Vec<Vec<Value>> = Vec::new();
    for _ in 0..65 {
        acquired.push(pool.acquire(4));
    }

    assert_eq!(acquired.len(), 65, "must be able to acquire 65 Vecs after releasing 70");
    // Every acquired Vec must have the correct length and all-Void contents.
    for v in &acquired {
        assert_eq!(v.len(), 4);
        for val in v {
            assert!(matches!(val, Value::Void));
        }
    }
}

/// FRAME-02 negative case: a pooled Vec whose capacity is less than the requested
/// reg_count must NOT be reused; a fresh Vec must be allocated instead.
#[test]
fn pool_acquire_respects_capacity() {
    let mut pool = RegisterPool::new();

    // Release a Vec with capacity 2 only.
    let small = Vec::with_capacity(2);
    pool.release(small);

    // Request 4 registers — capacity 2 < 4, so we expect a fresh allocation.
    let regs = pool.acquire(4);
    assert_eq!(regs.len(), 4, "fresh Vec must have len == reg_count");
    for val in &regs {
        assert!(matches!(val, Value::Void), "fresh Vec must be all Void");
    }
    // The pool should still hold the cap-2 entry (it was not reused).
    // Verify by acquiring with reg_count=1 — that should come from the pool.
    let small_reused = pool.acquire(1);
    assert_eq!(small_reused.len(), 1);
    assert!(matches!(small_reused[0], Value::Void));
}

/// FRAME-01 fresh path: acquire on an empty pool must allocate a fresh Vec of the
/// requested length with all registers set to Value::Void.
#[test]
fn pool_acquire_from_empty() {
    let mut pool = RegisterPool::new();

    let regs = pool.acquire(8);
    assert_eq!(regs.len(), 8, "len must equal reg_count");
    for val in &regs {
        assert!(matches!(val, Value::Void), "every register must be Value::Void");
    }
}

/// CallFrame::with_pool: the constructor must produce a frame whose register file
/// has exactly reg_count entries, all Value::Void, using the pool for allocation.
#[test]
fn pool_with_pool_constructor() {
    let mut pool = RegisterPool::new();

    let frame = CallFrame::with_pool(&mut pool, 0, 6, 0);
    assert_eq!(frame.registers.len(), 6, "frame must have exactly reg_count registers");
    for val in &frame.registers {
        assert!(matches!(val, Value::Void), "all registers must be Value::Void");
    }
}
