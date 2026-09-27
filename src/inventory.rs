//! Fixed-capacity player inventory for stackable planet resources and unique ceramics.

use bevy::prelude::Resource;

use crate::planet::ResourceType;

pub const INVENTORY_SLOT_COUNT: usize = 20;
pub const RESOURCE_STACK_LIMIT: u32 = 99;

/// Stable identity for one ceramic object. Ceramic item details are defined by
/// the crafting model; the inventory only needs to distinguish each object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CeramicObjectId(pub u64);

/// Contents of one inventory slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySlot {
    Empty,
    ResourceStack { resource: ResourceType, count: u32 },
    Ceramic(CeramicObjectId),
}

/// A 20-slot inventory. Resource stacks top out at 99; ceramics are never stacked.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    slots: [InventorySlot; INVENTORY_SLOT_COUNT],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [InventorySlot::Empty; INVENTORY_SLOT_COUNT],
        }
    }
}

impl Inventory {
    pub const fn slot_count(&self) -> usize {
        INVENTORY_SLOT_COUNT
    }

    /// Read a slot without changing the inventory. Out-of-range indices return `None`.
    pub fn slot(&self, index: usize) -> Option<InventorySlot> {
        self.slots.get(index).copied()
    }

    pub fn slots(&self) -> &[InventorySlot; INVENTORY_SLOT_COUNT] {
        &self.slots
    }

    /// Total quantity of a resource across all of its stacks.
    pub fn resource_count(&self, resource: ResourceType) -> u32 {
        self.slots
            .iter()
            .filter_map(|slot| match slot {
                InventorySlot::ResourceStack {
                    resource: stacked_resource,
                    count,
                } if *stacked_resource == resource => Some(*count),
                _ => None,
            })
            .sum()
    }

    /// Add as many units as fit, merging matching stacks before using empty slots.
    /// Returns the quantity that could not be added.
    pub fn add_resource(&mut self, resource: ResourceType, mut count: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        for slot in &mut self.slots {
            if let InventorySlot::ResourceStack {
                resource: stacked_resource,
                count: stack_count,
            } = slot
                && *stacked_resource == resource
            {
                let added = count.min(RESOURCE_STACK_LIMIT - *stack_count);
                *stack_count += added;
                count -= added;
                if count == 0 {
                    return 0;
                }
            }
        }

        for slot in &mut self.slots {
            if *slot == InventorySlot::Empty {
                let added = count.min(RESOURCE_STACK_LIMIT);
                *slot = InventorySlot::ResourceStack {
                    resource,
                    count: added,
                };
                count -= added;
                if count == 0 {
                    return 0;
                }
            }
        }
        count
    }

    /// Remove up to `count` units across matching stacks and return the quantity removed.
    pub fn remove_resource(&mut self, resource: ResourceType, mut count: u32) -> u32 {
        let requested = count;
        for slot in &mut self.slots {
            if let InventorySlot::ResourceStack {
                resource: stacked_resource,
                count: stack_count,
            } = slot
                && *stacked_resource == resource
            {
                let removed = count.min(*stack_count);
                *stack_count -= removed;
                count -= removed;
                if *stack_count == 0 {
                    *slot = InventorySlot::Empty;
                }
                if count == 0 {
                    break;
                }
            }
        }
        requested - count
    }

    /// Add a unique ceramic to one empty slot. Duplicate IDs and a full inventory fail.
    pub fn add_ceramic(&mut self, object: CeramicObjectId) -> bool {
        if self.slots.contains(&InventorySlot::Ceramic(object)) {
            return false;
        }
        let Some(slot) = self
            .slots
            .iter_mut()
            .find(|slot| **slot == InventorySlot::Empty)
        else {
            return false;
        };
        *slot = InventorySlot::Ceramic(object);
        true
    }

    /// Remove the ceramic with this ID, if present.
    pub fn remove_ceramic(&mut self, object: CeramicObjectId) -> bool {
        let Some(slot) = self
            .slots
            .iter_mut()
            .find(|slot| **slot == InventorySlot::Ceramic(object))
        else {
            return false;
        };
        *slot = InventorySlot::Empty;
        true
    }

    pub fn contains_ceramic(&self, object: CeramicObjectId) -> bool {
        self.slots.contains(&InventorySlot::Ceramic(object))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_resources_merge_until_stack_limit_then_use_another_slot() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.add_resource(ResourceType::CommonClay, 70), 0);
        assert_eq!(inventory.add_resource(ResourceType::CommonClay, 40), 0);
        assert_eq!(inventory.resource_count(ResourceType::CommonClay), 110);
        assert_eq!(
            inventory.slot(0),
            Some(InventorySlot::ResourceStack {
                resource: ResourceType::CommonClay,
                count: RESOURCE_STACK_LIMIT,
            })
        );
        assert_eq!(
            inventory.slot(1),
            Some(InventorySlot::ResourceStack {
                resource: ResourceType::CommonClay,
                count: 11,
            })
        );
    }

    #[test]
    fn full_inventory_reports_unadded_remainder() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.add_resource(ResourceType::Sand, 0), 0);
        assert!(
            inventory
                .slots()
                .iter()
                .all(|slot| *slot == InventorySlot::Empty)
        );
        assert_eq!(inventory.add_resource(ResourceType::Wood, 99 * 20), 0);
        assert_eq!(inventory.add_resource(ResourceType::Sand, 5), 5);
        assert_eq!(inventory.resource_count(ResourceType::Sand), 0);
        assert!(
            inventory
                .slots()
                .iter()
                .all(|slot| matches!(slot, InventorySlot::ResourceStack { count: 99, .. }))
        );
    }

    #[test]
    fn resource_removal_is_bounded_and_releases_empty_slots() {
        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceType::IronMineral, 120);
        assert_eq!(
            inventory.remove_resource(ResourceType::IronMineral, 105),
            105
        );
        assert_eq!(inventory.resource_count(ResourceType::IronMineral), 15);
        assert_eq!(inventory.remove_resource(ResourceType::IronMineral, 20), 15);
        assert_eq!(inventory.remove_resource(ResourceType::IronMineral, 1), 0);
        assert_eq!(inventory.slot(0), Some(InventorySlot::Empty));
        assert_eq!(inventory.slot(1), Some(InventorySlot::Empty));
    }

    #[test]
    fn each_ceramic_uses_one_slot_and_duplicate_or_missing_ids_are_safe() {
        let mut inventory = Inventory::default();
        let first = CeramicObjectId(1);
        let second = CeramicObjectId(2);
        assert!(inventory.add_ceramic(first));
        assert!(!inventory.add_ceramic(first));
        assert!(inventory.add_ceramic(second));
        assert_eq!(inventory.slot(0), Some(InventorySlot::Ceramic(first)));
        assert_eq!(inventory.slot(1), Some(InventorySlot::Ceramic(second)));
        assert!(inventory.contains_ceramic(first));
        assert!(!inventory.remove_ceramic(CeramicObjectId(99)));
        assert!(inventory.remove_ceramic(first));
        assert!(!inventory.contains_ceramic(first));
    }

    #[test]
    fn ceramics_fail_cleanly_when_all_slots_are_occupied() {
        let mut inventory = Inventory::default();
        for id in 0..INVENTORY_SLOT_COUNT as u64 {
            assert!(inventory.add_ceramic(CeramicObjectId(id)));
        }
        assert!(!inventory.add_ceramic(CeramicObjectId(100)));
        assert_eq!(inventory.slot(INVENTORY_SLOT_COUNT), None);
    }
}
