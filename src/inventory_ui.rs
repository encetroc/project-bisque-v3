//! Compact inventory and machine status panels backed by the domain model.

use bevy::prelude::*;

use crate::{
    ceramics::CeramicItem,
    inventory::{INVENTORY_SLOT_COUNT, Inventory, InventorySlot},
    workbench::CraftedCeramics,
};

#[derive(Component)]
struct InventoryPanel;

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_inventory_panel)
            .add_systems(Update, refresh_inventory_panel);
    }
}

fn spawn_inventory_panel(mut commands: Commands) {
    commands.spawn((
        InventoryPanel,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            max_width: px(360),
            ..default()
        },
    ));
}

fn ceramic_description(item: &CeramicItem) -> String {
    format!(
        "{:?} #{:03} — {:?} clay, {:?} glaze, {:?}",
        item.form(),
        item.id.0,
        item.clay(),
        item.glaze(),
        item.state(),
    )
}

fn inventory_panel_text(inventory: &Inventory, ceramics: &[CeramicItem]) -> String {
    let mut lines = Vec::with_capacity(INVENTORY_SLOT_COUNT + 1);
    lines.push(format!("Inventory ({INVENTORY_SLOT_COUNT} slots)"));
    for (index, slot) in inventory.slots().iter().enumerate() {
        let description = match slot {
            InventorySlot::Empty => "Empty".to_owned(),
            InventorySlot::ResourceStack { resource, count } => {
                format!("{resource:?} ×{count}")
            }
            InventorySlot::Ceramic(id) => ceramics
                .iter()
                .find(|item| item.id == *id)
                .map(ceramic_description)
                .unwrap_or_else(|| format!("Ceramic #{:03} — details unavailable", id.0)),
        };
        lines.push(format!("{:02}. {description}", index + 1));
    }
    lines.join("\n")
}

fn refresh_inventory_panel(
    inventory: Res<Inventory>,
    ceramics: Res<CraftedCeramics>,
    mut panels: Query<&mut Text, With<InventoryPanel>>,
) {
    if !inventory.is_changed() && !ceramics.is_changed() {
        return;
    }
    let text = inventory_panel_text(&inventory, &ceramics.items);
    for mut panel in &mut panels {
        panel.0.clone_from(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ceramics::{CeramicForm, CeramicItemTemplate, ClayMaterial, Glaze, ProcessingState},
        inventory::CeramicObjectId,
        planet::ResourceType,
        workbench::CraftedCeramics,
    };

    #[test]
    fn inventory_panel_matches_stacks_and_describes_unique_ceramics() {
        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceType::CommonClay, 12);
        let id = CeramicObjectId(7);
        inventory.add_ceramic(id);
        let ceramic = CeramicItemTemplate {
            form: CeramicForm::Bowl,
            clay: ClayMaterial::Red,
            glaze: Glaze::Blue,
            state: ProcessingState::Dry,
        }
        .instantiate(id);

        let text = inventory_panel_text(&inventory, &[ceramic]);
        assert!(text.contains("Inventory (20 slots)"));
        assert!(text.contains("CommonClay ×12"));
        assert!(text.contains("Bowl #007 — Red clay, Blue glaze, Dry"));
        assert_eq!(text.lines().count(), INVENTORY_SLOT_COUNT + 1);
    }

    #[test]
    fn panel_refreshes_from_ecs_inventory_without_a_window() {
        let mut app = App::new();
        app.init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .add_systems(Update, refresh_inventory_panel);
        app.world_mut().spawn((InventoryPanel, Text::new("")));
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_resource(ResourceType::Wood, 8);
        app.update();

        let text = app
            .world_mut()
            .query_filtered::<&Text, With<InventoryPanel>>()
            .single(app.world())
            .unwrap();
        assert!(text.0.contains("Wood ×8"));
    }
}
