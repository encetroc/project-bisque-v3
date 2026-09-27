use bevy::prelude::*;

pub mod camera_follow;
pub mod ceramic_visuals;
pub mod ceramics;
pub mod drying_rack;
pub mod economy;
pub mod game_clock;
pub mod interaction;
pub mod inventory;
pub mod kiln;
pub mod placement;
pub mod planet;
pub mod planet_surface;
pub mod player_movement;
pub mod resource_nodes;
pub mod studio;
pub mod surface_transform;
pub mod workbench;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(game_clock::GameClockPlugin)
        .add_plugins(economy::EconomyPlugin)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .add_plugins(ceramic_visuals::CeramicVisualPlugin)
        .add_plugins(resource_nodes::ResourceNodePlugin)
        .add_plugins(interaction::InteractionPlugin)
        .add_plugins(workbench::WorkbenchPlugin)
        .add_plugins(drying_rack::DryingRackPlugin)
        .add_plugins(kiln::KilnPlugin)
        .add_plugins(studio::StudioPlugin)
        .add_plugins(placement::PlacementPlugin)
        .add_plugins(player_movement::PlayerMovementPlugin)
        .add_plugins(camera_follow::SurfaceCameraFollowPlugin)
        .run();
}
