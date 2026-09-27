//! Primitive NPC characters and their named, surface-aligned properties.

use bevy::prelude::*;

use crate::{
    economy::Merchant,
    interaction::Interactable,
    planet::{DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface},
    surface_transform::{SurfaceLocation, surface_transform},
};

/// The three named people who inhabit the starter world.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcCharacter {
    Baker,
    Carpenter,
    Merchant,
}

/// A named NPC destination/property in the world.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcProperty {
    Bakery,
    Workshop,
    GeneralStore,
}

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_npc_world);
    }
}

struct NpcDefinition {
    character: NpcCharacter,
    character_name: &'static str,
    property: NpcProperty,
    property_name: &'static str,
    interaction_prompt: &'static str,
    character_tile: (u8, u8),
    property_tile: (u8, u8),
    body_color: Color,
    accessory_color: Color,
}

const NPCS: [NpcDefinition; 3] = [
    NpcDefinition {
        character: NpcCharacter::Baker,
        character_name: "Baker",
        property: NpcProperty::Bakery,
        property_name: "Bakery",
        interaction_prompt: "Talk to the Baker",
        character_tile: (8, 12),
        property_tile: (9, 12),
        body_color: Color::srgb(0.92, 0.89, 0.81),
        accessory_color: Color::srgb(0.96, 0.96, 0.91),
    },
    NpcDefinition {
        character: NpcCharacter::Carpenter,
        character_name: "Carpenter",
        property: NpcProperty::Workshop,
        property_name: "Workshop",
        interaction_prompt: "Talk to the Carpenter",
        character_tile: (11, 12),
        property_tile: (12, 12),
        body_color: Color::srgb(0.43, 0.25, 0.14),
        accessory_color: Color::srgb(0.29, 0.17, 0.09),
    },
    NpcDefinition {
        character: NpcCharacter::Merchant,
        character_name: "Merchant",
        property: NpcProperty::GeneralStore,
        property_name: "General Store",
        interaction_prompt: "Trade with the Merchant",
        character_tile: (14, 12),
        property_tile: (15, 12),
        body_color: Color::srgb(0.13, 0.36, 0.76),
        accessory_color: Color::srgb(0.30, 0.58, 0.91),
    },
];

fn spawn_npc_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(Capsule3d::new(0.38, 0.85));
    let head_mesh = meshes.add(Sphere::new(0.34));
    let chef_hat_mesh = meshes.add(Cylinder::new(0.22, 0.38));
    let toolbox_mesh = meshes.add(Cuboid::new(0.55, 0.48, 0.5));
    let merchant_hat_mesh = meshes.add(Sphere::new(0.19));
    let wood = materials.add(Color::srgb(0.48, 0.30, 0.16));
    let roof = materials.add(Color::srgb(0.69, 0.30, 0.18));
    let pale = materials.add(Color::srgb(0.86, 0.75, 0.54));
    let blue = materials.add(Color::srgb(0.24, 0.42, 0.60));

    for definition in NPCS {
        let body_material = materials.add(definition.body_color);
        let accessory_material = materials.add(definition.accessory_color);
        let character_transform = location_transform(definition.character_tile);
        let character = commands
            .spawn((
                Name::new(definition.character_name),
                definition.character,
                character_transform,
                Interactable::new(definition.interaction_prompt),
                Visibility::default(),
            ))
            .id();
        commands.entity(character).with_children(|children| {
            children.spawn((
                Name::new(format!("{} body", definition.character_name)),
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(body_material),
                Transform::from_xyz(0.0, 0.72, 0.0),
            ));
            children.spawn((
                Name::new(format!("{} head", definition.character_name)),
                Mesh3d(head_mesh.clone()),
                MeshMaterial3d(materials.add(Color::srgb(0.83, 0.66, 0.52))),
                Transform::from_xyz(0.0, 1.48, 0.0),
            ));
            let (mesh, position, name) = match definition.character {
                NpcCharacter::Baker => {
                    (chef_hat_mesh.clone(), Vec3::new(0.0, 1.9, 0.0), "Chef hat")
                }
                NpcCharacter::Carpenter => (
                    toolbox_mesh.clone(),
                    Vec3::new(0.0, 0.75, 0.43),
                    "Toolbox backpack",
                ),
                NpcCharacter::Merchant => (
                    merchant_hat_mesh.clone(),
                    Vec3::new(0.0, 1.88, 0.0),
                    "Merchant hat",
                ),
            };
            children.spawn((
                Name::new(name),
                Mesh3d(mesh),
                MeshMaterial3d(accessory_material),
                Transform::from_translation(position),
            ));
        });

        let property_transform = location_transform(definition.property_tile);
        let property = commands
            .spawn((
                Name::new(definition.property_name),
                definition.property,
                property_transform,
                Interactable::new(format!("Visit the {}", definition.property_name)),
                Visibility::default(),
            ))
            .id();
        commands.entity(property).with_children(|children| {
            let (wall_material, roof_material) = match definition.property {
                NpcProperty::Bakery => (pale.clone(), roof.clone()),
                NpcProperty::Workshop => (wood.clone(), roof.clone()),
                NpcProperty::GeneralStore => (pale.clone(), blue.clone()),
            };
            // A simple, open-fronted shell: side walls, rear wall, and a pitched-roof cue.
            children.spawn((
                Name::new(format!("{} walls", definition.property_name)),
                Mesh3d(meshes.add(Cuboid::new(2.2, 1.8, 0.16))),
                MeshMaterial3d(wall_material.clone()),
                Transform::from_xyz(0.0, 0.9, -0.9),
            ));
            for x in [-1.02, 1.02] {
                children.spawn((
                    Name::new(format!("{} side wall", definition.property_name)),
                    Mesh3d(meshes.add(Cuboid::new(0.16, 1.8, 1.8))),
                    MeshMaterial3d(wall_material.clone()),
                    Transform::from_xyz(x, 0.9, 0.0),
                ));
            }
            children.spawn((
                Name::new(format!("{} roof", definition.property_name)),
                Mesh3d(meshes.add(Cuboid::new(2.55, 0.18, 2.35))),
                MeshMaterial3d(roof_material),
                Transform::from_xyz(0.0, 1.92, -0.02).with_rotation(Quat::from_rotation_z(0.12)),
            ));
        });

        if definition.character == NpcCharacter::Merchant {
            commands.entity(character).insert(Merchant);
        }
    }
}

fn location_transform(tile: (u8, u8)) -> Transform {
    let coordinate = TileCoordinate::new(PlanetFace::PositiveY, tile.0, tile.1)
        .expect("NPC location tiles are within the authored planet face");
    let surface = sample_tile_surface(&PlanetTile::new(coordinate), DEFAULT_PLANET_RADIUS)
        .expect("the configured planet radius is valid");
    surface_transform(
        SurfaceLocation::new(surface.normal, 0.04),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("NPC locations have valid tangent orientations")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_world_contains_exactly_three_interactable_surface_aligned_npcs_and_properties() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.add_plugins(NpcPlugin);
        app.update();

        let world = app.world_mut();
        let mut characters = world.query::<(&Name, &NpcCharacter, &Transform, &Interactable)>();
        let characters: Vec<_> = characters.iter(world).collect();
        assert_eq!(characters.len(), 3);
        for expected in ["Baker", "Carpenter", "Merchant"] {
            assert_eq!(
                characters
                    .iter()
                    .filter(|(name, _, _, _)| name.as_str() == expected)
                    .count(),
                1
            );
        }
        for (_, _, transform, interactable) in characters {
            assert!(interactable.range > 0.0);
            let up = transform.rotation * Vec3::Y;
            assert!(up.dot(transform.translation.normalize()) > 0.999);
        }

        let mut properties = world.query::<(&Name, &NpcProperty, &Transform, &Interactable)>();
        let properties: Vec<_> = properties.iter(world).collect();
        assert_eq!(properties.len(), 3);
        for expected in ["Bakery", "Workshop", "General Store"] {
            assert_eq!(
                properties
                    .iter()
                    .filter(|(name, _, _, _)| name.as_str() == expected)
                    .count(),
                1
            );
        }
        for (_, _, transform, interactable) in properties {
            assert!(interactable.range > 0.0);
            let up = transform.rotation * Vec3::Y;
            assert!(up.dot(transform.translation.normalize()) > 0.999);
        }

        let mut merchants = world.query_filtered::<Entity, With<Merchant>>();
        assert_eq!(merchants.iter(world).count(), 1);
    }
}
