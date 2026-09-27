use bevy::prelude::*;

pub mod camera_follow;
pub mod planet;
pub mod planet_surface;
pub mod player_movement;
pub mod resource_nodes;
pub mod surface_transform;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .add_plugins(resource_nodes::ResourceNodePlugin)
        .add_plugins(player_movement::PlayerMovementPlugin)
        .add_plugins(camera_follow::SurfaceCameraFollowPlugin)
        .run();
}
