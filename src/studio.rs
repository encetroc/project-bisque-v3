//! Open-air starter studio layout on the planet surface.

use bevy::prelude::*;

use crate::{
    planet::{DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface},
    surface_transform::{SurfaceLocation, surface_transform},
};

pub struct StudioPlugin;

impl Plugin for StudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_studio);
    }
}

fn spawn_studio(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14).expect("studio tile is within the face"),
    );
    let surface = sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
        .expect("the configured planet radius is valid");
    let transform = surface_transform(
        SurfaceLocation::new(surface.normal, 0.05),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("the studio's tangent orientation is valid");

    let timber = materials.add(Color::srgb(0.38, 0.23, 0.13));
    let roof = materials.add(Color::srgb(0.66, 0.31, 0.19));
    let stone = materials.add(Color::srgb(0.48, 0.47, 0.40));
    let marker = materials.add(Color::srgba(0.92, 0.76, 0.22, 0.7));
    let storage = materials.add(Color::srgb(0.30, 0.43, 0.29));

    let studio = commands
        .spawn((
            Name::new("Open-air starter studio"),
            transform,
            Visibility::default(),
        ))
        .id();
    commands.entity(studio).with_children(|children| {
        // Low stone pad visually anchors the simple, roofed-but-open work area.
        children.spawn((
            Name::new("Studio stone pad"),
            Mesh3d(meshes.add(Cuboid::new(12.0, 0.16, 5.0))),
            MeshMaterial3d(stone),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        for x in [-5.6, 5.6] {
            for z in [-2.1, 2.1] {
                children.spawn((
                    Name::new("Shed support post"),
                    Mesh3d(meshes.add(Cuboid::new(0.22, 3.0, 0.22))),
                    MeshMaterial3d(timber.clone()),
                    Transform::from_xyz(x, 1.5, z),
                ));
            }
        }
        children.spawn((
            Name::new("Shed roof"),
            Mesh3d(meshes.add(Cuboid::new(12.4, 0.28, 5.4))),
            MeshMaterial3d(roof),
            Transform::from_xyz(0.0, 3.0, 0.0),
        ));
        // Storage is represented by a visible marker until storage gameplay exists.
        children.spawn((
            Name::new("Studio storage marker"),
            Mesh3d(meshes.add(Cuboid::new(1.1, 0.9, 0.8))),
            MeshMaterial3d(storage),
            Transform::from_xyz(4.8, 0.55, 1.55),
        ));
        // Clear, bright floor patch identifies where the player can arrange ceramics.
        children.spawn((
            Name::new("Studio display and placement area"),
            Mesh3d(meshes.add(Cylinder::new(1.25, 0.035))),
            MeshMaterial3d(marker),
            Transform::from_xyz(0.0, 0.105, -1.05),
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_startup_builds_rigid_surface_aligned_studio_and_markers() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.add_plugins(StudioPlugin);
        app.update();

        let world = app.world_mut();
        let studio_entity = {
            let mut query = world.query::<(Entity, &Name, &Transform)>();
            query
                .iter(world)
                .find(|(_, name, _)| name.as_str() == "Open-air starter studio")
                .map(|(entity, _, _)| entity)
                .expect("studio root is spawned")
        };
        let studio_transform = *world.get::<Transform>(studio_entity).unwrap();
        assert!(studio_transform.translation.is_finite());
        let expected_up = studio_transform.translation.normalize();
        assert!((studio_transform.rotation * Vec3::Y).dot(expected_up) > 0.999);

        let mut names = world.query::<&Name>();
        let names: Vec<_> = names.iter(world).map(Name::as_str).collect();
        assert!(names.contains(&"Shed roof"));
        assert!(names.contains(&"Studio storage marker"));
        assert!(names.contains(&"Studio display and placement area"));
        assert_eq!(
            names
                .iter()
                .filter(|name| **name == "Shed support post")
                .count(),
            4
        );
    }
}
