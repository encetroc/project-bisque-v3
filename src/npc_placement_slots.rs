//! Bounded, predefined placement slots for persistent NPC-property ceramics.

use bevy::prelude::*;

use crate::ceramics::{CeramicForm, CeramicItem};

/// A single property display position with explicit category eligibility and capacity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcPlacementSlot {
    pub index: usize,
    pub allowed_forms: [CeramicForm; 3],
    pub occupied_by: Option<CeramicItem>,
}

impl NpcPlacementSlot {
    pub const fn bakery(index: usize) -> Self {
        Self {
            index,
            allowed_forms: CeramicForm::ALL,
            occupied_by: None,
        }
    }

    /// Occupy this slot once, and only with an allowed ceramic form.
    pub fn occupy(&mut self, item: CeramicItem) -> Result<(), SlotError> {
        if !self.allowed_forms.contains(&item.form()) {
            return Err(SlotError::IneligibleForm);
        }
        if self.occupied_by.is_some() {
            return Err(SlotError::Occupied);
        }
        self.occupied_by = Some(item);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotError {
    IneligibleForm,
    Occupied,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ceramics::{CeramicItemTemplate, ClayMaterial, Glaze, ProcessingState},
        inventory::CeramicObjectId,
    };

    fn item(id: u64, form: CeramicForm) -> CeramicItem {
        CeramicItemTemplate {
            form,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state: ProcessingState::Fired,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn slots_enforce_allowed_categories_and_single_occupancy() {
        let mut cup_only = NpcPlacementSlot {
            allowed_forms: [CeramicForm::Cup; 3],
            ..NpcPlacementSlot::bakery(0)
        };
        assert_eq!(
            cup_only.occupy(item(1, CeramicForm::Bowl)),
            Err(SlotError::IneligibleForm)
        );
        assert_eq!(cup_only.occupied_by, None);
        assert_eq!(cup_only.occupy(item(2, CeramicForm::Cup)), Ok(()));
        assert_eq!(
            cup_only.occupy(item(3, CeramicForm::Cup)),
            Err(SlotError::Occupied)
        );
        assert_eq!(cup_only.occupied_by, Some(item(2, CeramicForm::Cup)));
    }
}
