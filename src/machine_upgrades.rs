//! Shared payment rules for the one-time studio machine upgrades.

use crate::{economy::Wallet, inventory::Inventory, planet::ResourceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineUpgradeCost {
    pub coins: u64,
    pub material: ResourceType,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineUpgradeError {
    AlreadyUpgraded,
    InsufficientCoins,
    InsufficientMaterials,
}

/// Charge both parts of an upgrade price together, changing neither on failure.
pub fn pay_machine_upgrade(
    upgraded: bool,
    cost: MachineUpgradeCost,
    inventory: &mut Inventory,
    wallet: &mut Wallet,
) -> Result<(), MachineUpgradeError> {
    if upgraded {
        return Err(MachineUpgradeError::AlreadyUpgraded);
    }
    if wallet.coins < cost.coins {
        return Err(MachineUpgradeError::InsufficientCoins);
    }
    if inventory.resource_count(cost.material) < cost.quantity {
        return Err(MachineUpgradeError::InsufficientMaterials);
    }

    let removed = inventory.remove_resource(cost.material, cost.quantity);
    debug_assert_eq!(removed, cost.quantity);
    wallet.coins -= cost.coins;
    Ok(())
}
