use bevy::prelude::*;

pub mod planet;
pub mod planet_surface;
pub mod player_movement;
pub mod surface_transform;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .add_plugins(player_movement::PlayerMovementPlugin)
        .run();
}
