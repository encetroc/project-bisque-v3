use bevy::prelude::*;

pub mod planet;
pub mod planet_surface;
pub mod surface_transform;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .run();
}
