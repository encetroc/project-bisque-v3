use bevy::prelude::*;

pub mod planet;
pub mod planet_surface;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(planet_surface::PlanetSurfacePlugin)
        .run();
}
