use bevy::prelude::*;

pub mod camera_follow;
pub mod ceramic_visuals;
pub mod ceramics;
pub mod day_time_hud;
pub mod drying_rack;
pub mod economy;
pub mod game_clock;
pub mod interaction;
pub mod inventory;
pub mod inventory_ui;
pub mod kiln;
pub mod machine_upgrades;
pub mod npc_placement_slots;
pub mod npc_production;
pub mod npc_property_upgrade;
pub mod npc_requests;
pub mod npcs;
pub mod objectives;
pub mod placement;
pub mod planet;
pub mod planet_surface;
pub mod player_movement;
pub mod resource_nodes;
pub mod save_game;
pub mod studio;
pub mod surface_transform;
pub mod workbench;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(game_clock::GameClockPlugin)
        .add_plugins(day_time_hud::DayTimeHudPlugin)
        .add_plugins(economy::EconomyPlugin)
        .add_plugins(objectives::ObjectivePlugin)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .add_plugins(ceramic_visuals::CeramicVisualPlugin)
        .add_plugins(resource_nodes::ResourceNodePlugin)
        .add_plugins(interaction::InteractionPlugin)
        .add_plugins(workbench::WorkbenchPlugin)
        .add_plugins(inventory_ui::InventoryUiPlugin)
        .add_plugins(drying_rack::DryingRackPlugin)
        .add_plugins(kiln::KilnPlugin)
        .add_plugins(studio::StudioPlugin)
        .add_plugins(npcs::NpcPlugin)
        .add_plugins(npc_requests::NpcRequestPlugin)
        .add_plugins(npc_property_upgrade::NpcPropertyUpgradePlugin)
        .add_plugins(npc_production::NpcProductionPlugin)
        .add_plugins(placement::PlacementPlugin)
        .add_plugins(player_movement::PlayerMovementPlugin)
        .add_plugins(camera_follow::SurfaceCameraFollowPlugin)
        .add_plugins(save_game::SaveGamePlugin)
        .run();
}
