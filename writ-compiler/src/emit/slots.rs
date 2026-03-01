//! CALL_VIRT slot assignment from contract declaration order.
//!
//! The critical invariant: slot numbers come from the order methods
//! appear in the ContractDef declaration, NOT from impl block traversal.

use super::module_builder::ModuleBuilder;

/// Assign CALL_VIRT slot indices to ContractMethod rows.
///
/// For each ContractDef, walk its ContractMethod rows (in declaration order)
/// and assign slot indices 0, 1, 2, ...
pub fn assign_vtable_slots(builder: &mut ModuleBuilder) {
    let contract_count = builder.contract_def_count();
    for contract_idx in 0..contract_count {
        let range = builder.contract_method_range(contract_idx);
        for (slot, cm_idx) in range.enumerate() {
            builder.set_contract_method_slot(cm_idx, slot as u16);
        }
    }
}
