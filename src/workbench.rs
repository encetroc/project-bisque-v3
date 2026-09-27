//! Workbench interaction, recipe selection, and transactional greenware crafting.

use bevy::prelude::*;

use crate::{
    ceramics::{CeramicForm, CeramicItem, ClayMaterial, Glaze, recipe_for},
    interaction::InteractionRequested,
    inventory::{CeramicObjectId, Inventory},
};

/// Marks an entity as a level-one shaping workbench.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Workbench;

/// Player's current recipe selection while using a workbench.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchSelection {
    pub active: bool,
    pub form: CeramicForm,
    pub clay: ClayMaterial,
    pub glaze: Glaze,
}

impl Default for WorkbenchSelection {
    fn default() -> Self {
        Self {
            active: false,
            form: CeramicForm::Cup,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
        }
    }
}

/// Allocates stable IDs and keeps the full data for crafted items.
#[derive(Resource, Debug, Default)]
pub struct CraftedCeramics {
    next_id: u64,
    pub items: Vec<CeramicItem>,
}

#[derive(Resource, Debug, Default)]
struct WorkbenchFeedback(Option<String>);

#[derive(Component)]
struct WorkbenchHelpText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftError {
    FormLocked,
    MissingClay,
    InventoryFull,
    IdentityExhausted,
}

/// Craft the selected recipe atomically: inventory and item identity only change on success.
pub fn craft_greenware(
    inventory: &mut Inventory,
    crafted: &mut CraftedCeramics,
    form: CeramicForm,
    clay: ClayMaterial,
    glaze: Glaze,
) -> Result<CeramicObjectId, CraftError> {
    if !matches!(form, CeramicForm::Cup | CeramicForm::Bowl) {
        return Err(CraftError::FormLocked);
    }
    let recipe = recipe_for(form, clay, glaze).expect("all supported recipes are catalogued");
    if inventory.resource_count(recipe.input.resource) < recipe.input.quantity {
        return Err(CraftError::MissingClay);
    }

    let mut next_id = crafted.next_id;
    let id = loop {
        next_id = next_id
            .checked_add(1)
            .ok_or(CraftError::IdentityExhausted)?;
        let candidate = CeramicObjectId(next_id);
        if !inventory.contains_ceramic(candidate) {
            break candidate;
        }
    };
    let mut updated_inventory = inventory.clone();
    assert_eq!(
        updated_inventory.remove_resource(recipe.input.resource, recipe.input.quantity),
        recipe.input.quantity,
        "resource count was checked above"
    );
    if !updated_inventory.add_ceramic(id) {
        return Err(CraftError::InventoryFull);
    }

    *inventory = updated_inventory;
    crafted.next_id = id.0;
    crafted.items.push(recipe.output.instantiate(id));
    Ok(id)
}

pub struct WorkbenchPlugin;

impl Plugin for WorkbenchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<WorkbenchSelection>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<WorkbenchFeedback>()
            .add_systems(Startup, spawn_workbench_help)
            .add_systems(
                Update,
                (
                    begin_workbench_use,
                    control_workbench,
                    update_workbench_help,
                )
                    .chain(),
            );
    }
}

fn spawn_workbench_help(mut commands: Commands) {
    commands.spawn((
        WorkbenchHelpText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(48),
            left: px(16),
            ..default()
        },
    ));
}

fn begin_workbench_use(
    mut requests: MessageReader<InteractionRequested>,
    workbenches: Query<(), With<Workbench>>,
    mut selection: ResMut<WorkbenchSelection>,
) {
    for request in requests.read() {
        if workbenches.contains(request.target) {
            selection.active = true;
        }
    }
}

fn control_workbench(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<WorkbenchSelection>,
    mut inventory: ResMut<Inventory>,
    mut crafted: ResMut<CraftedCeramics>,
    mut feedback: ResMut<WorkbenchFeedback>,
) {
    if !selection.active {
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        selection.active = false;
        feedback.0 = None;
        return;
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        selection.form = CeramicForm::Cup;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        selection.form = CeramicForm::Bowl;
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        selection.clay = ClayMaterial::Common;
    } else if keyboard.just_pressed(KeyCode::KeyR) {
        selection.clay = ClayMaterial::Red;
    } else if keyboard.just_pressed(KeyCode::KeyP) {
        selection.clay = ClayMaterial::Pale;
    }
    if keyboard.just_pressed(KeyCode::Digit0) {
        selection.glaze = Glaze::None;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        selection.glaze = Glaze::Blue;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        selection.glaze = Glaze::Green;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        selection.glaze = Glaze::White;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        let WorkbenchSelection {
            form, clay, glaze, ..
        } = *selection;
        feedback.0 = Some(
            match craft_greenware(&mut inventory, &mut crafted, form, clay, glaze) {
                Ok(_) => "Greenware crafted!".to_owned(),
                Err(CraftError::FormLocked) => "Vase is locked.".to_owned(),
                Err(CraftError::MissingClay) => "Not enough selected clay (3 required).".to_owned(),
                Err(CraftError::InventoryFull) => "Inventory is full.".to_owned(),
                Err(CraftError::IdentityExhausted) => "Cannot create another ceramic.".to_owned(),
            },
        );
    }
}

fn update_workbench_help(
    selection: Res<WorkbenchSelection>,
    feedback: Res<WorkbenchFeedback>,
    mut labels: Query<&mut Text, With<WorkbenchHelpText>>,
) {
    if !selection.is_changed() && !feedback.is_changed() {
        return;
    }
    let message = if !selection.active {
        String::new()
    } else {
        format!(
            "Workbench — 1 Cup / 2 Bowl | C Common / R Red / P Pale clay | 0 None / 3 Blue / 4 Green / 5 White glaze | Enter Craft | Esc Close\nSelected: {:?}, {:?} clay, {:?} glaze{}",
            selection.form,
            selection.clay,
            selection.glaze,
            feedback
                .0
                .as_ref()
                .map_or_else(String::new, |text| format!(" — {text}")),
        )
    };
    for mut label in &mut labels {
        label.0.clone_from(&message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ceramics::ProcessingState, planet::ResourceType};

    fn stock(inventory: &mut Inventory, clay: ClayMaterial, quantity: u32) {
        assert_eq!(inventory.add_resource(clay.resource(), quantity), 0);
    }

    #[test]
    fn successful_craft_consumes_clay_and_adds_greenware_to_inventory() {
        let mut inventory = Inventory::default();
        stock(&mut inventory, ClayMaterial::Common, 5);
        let mut crafted = CraftedCeramics::default();

        let id = craft_greenware(
            &mut inventory,
            &mut crafted,
            CeramicForm::Bowl,
            ClayMaterial::Common,
            Glaze::Blue,
        )
        .unwrap();

        assert_eq!(inventory.resource_count(ResourceType::CommonClay), 2);
        assert!(inventory.contains_ceramic(id));
        assert_eq!(crafted.items.len(), 1);
        assert_eq!(crafted.items[0].id, id);
        assert_eq!(crafted.items[0].form(), CeramicForm::Bowl);
        assert_eq!(crafted.items[0].clay(), ClayMaterial::Common);
        assert_eq!(crafted.items[0].glaze(), Glaze::Blue);
        assert_eq!(crafted.items[0].state(), ProcessingState::Greenware);
    }

    #[test]
    fn missing_clay_does_not_change_inventory_or_output() {
        let mut inventory = Inventory::default();
        stock(&mut inventory, ClayMaterial::Common, 2);
        let before = inventory.clone();
        let mut crafted = CraftedCeramics::default();

        assert_eq!(
            craft_greenware(
                &mut inventory,
                &mut crafted,
                CeramicForm::Cup,
                ClayMaterial::Common,
                Glaze::None,
            ),
            Err(CraftError::MissingClay)
        );
        assert_eq!(inventory, before);
        assert!(crafted.items.is_empty());
    }

    #[test]
    fn full_inventory_does_not_consume_clay_or_create_greenware() {
        let mut inventory = Inventory::default();
        stock(&mut inventory, ClayMaterial::Common, 4);
        for id in 100..119 {
            assert!(inventory.add_ceramic(CeramicObjectId(id)));
        }
        let before = inventory.clone();
        let mut crafted = CraftedCeramics::default();

        assert_eq!(
            craft_greenware(
                &mut inventory,
                &mut crafted,
                CeramicForm::Cup,
                ClayMaterial::Common,
                Glaze::None,
            ),
            Err(CraftError::InventoryFull)
        );
        assert_eq!(inventory, before);
        assert!(crafted.items.is_empty());
    }

    #[test]
    fn vase_is_locked_at_level_one_without_consuming_inputs() {
        let mut inventory = Inventory::default();
        stock(&mut inventory, ClayMaterial::Red, 3);
        let before = inventory.clone();
        let mut crafted = CraftedCeramics::default();

        assert_eq!(
            craft_greenware(
                &mut inventory,
                &mut crafted,
                CeramicForm::Vase,
                ClayMaterial::Red,
                Glaze::None,
            ),
            Err(CraftError::FormLocked)
        );
        assert_eq!(inventory, before);
        assert!(crafted.items.is_empty());
    }

    #[test]
    fn workbench_selection_responds_to_controls_headlessly() {
        let mut app = App::new();
        app.init_resource::<WorkbenchSelection>()
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<WorkbenchFeedback>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, control_workbench);
        app.world_mut().resource_mut::<WorkbenchSelection>().active = true;
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.press(KeyCode::Digit2);
        keyboard.press(KeyCode::KeyR);
        keyboard.press(KeyCode::Digit3);
        app.update();
        let selected = *app.world().resource::<WorkbenchSelection>();
        assert_eq!(selected.form, CeramicForm::Bowl);
        assert_eq!(selected.clay, ClayMaterial::Red);
        assert_eq!(selected.glaze, Glaze::Blue);
    }
}
