//! Primitive NPC characters and their named, surface-aligned properties.

use bevy::prelude::*;

use crate::{
    economy::Merchant,
    game_clock::GameClock,
    interaction::Interactable,
    planet::{
        DEFAULT_PLANET_RADIUS, FaceOrientation, PlanetCoordinate, PlanetFace, PlanetTile,
        TileCoordinate, sample_tile_surface,
    },
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

/// A deterministic phase in an NPC's authored daily routine.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcScheduleState {
    Home,
    Travel,
    Work,
    Social,
}

/// Named authored locations used by NPC schedules and exposed for debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcDestination {
    Home,
    Work,
    Social,
}

/// Inspectable schedule snapshot attached to each NPC entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcScheduleDebug {
    pub state: NpcScheduleState,
    pub destination: NpcDestination,
    /// Index into the active authored route (zero while stationary).
    pub waypoint_index: usize,
}

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_npc_world)
            .add_systems(Update, update_npc_schedules);
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

const SOCIAL_TILE: (u8, u8) = (12, 16);

#[derive(Clone, Copy)]
struct SchedulePhase {
    state: NpcScheduleState,
    destination: NpcDestination,
    route: [(u8, u8); 3],
    route_len: usize,
    start_minute: f64,
    end_minute: f64,
}

fn schedule_phase(character: NpcCharacter, minute: f64) -> SchedulePhase {
    let (home, work) = match character {
        NpcCharacter::Baker => ((8, 12), (9, 12)),
        NpcCharacter::Carpenter => ((11, 12), (12, 12)),
        NpcCharacter::Merchant => ((14, 12), (15, 12)),
    };
    let social = SOCIAL_TILE;
    let stationary = [(0, 0); 3];
    let phases = [
        SchedulePhase {
            state: NpcScheduleState::Home,
            destination: NpcDestination::Home,
            route: stationary,
            route_len: 0,
            start_minute: 0.0,
            end_minute: 8.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Work,
            route: [home, (8, 13), work],
            route_len: 3,
            start_minute: 8.0 * 60.0,
            end_minute: 9.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Work,
            destination: NpcDestination::Work,
            route: stationary,
            route_len: 0,
            start_minute: 9.0 * 60.0,
            end_minute: 12.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Social,
            route: [work, (10, 14), social],
            route_len: 3,
            start_minute: 12.0 * 60.0,
            end_minute: 12.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Social,
            destination: NpcDestination::Social,
            route: stationary,
            route_len: 0,
            start_minute: 12.5 * 60.0,
            end_minute: 13.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Work,
            route: [social, (13, 14), work],
            route_len: 3,
            start_minute: 13.5 * 60.0,
            end_minute: 14.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Work,
            destination: NpcDestination::Work,
            route: stationary,
            route_len: 0,
            start_minute: 14.0 * 60.0,
            end_minute: 18.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Social,
            route: [work, (13, 14), social],
            route_len: 3,
            start_minute: 18.0 * 60.0,
            end_minute: 18.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Social,
            destination: NpcDestination::Social,
            route: stationary,
            route_len: 0,
            start_minute: 18.5 * 60.0,
            end_minute: 19.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Home,
            route: [social, (10, 14), home],
            route_len: 3,
            start_minute: 19.5 * 60.0,
            end_minute: 21.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Home,
            destination: NpcDestination::Home,
            route: stationary,
            route_len: 0,
            start_minute: 21.0 * 60.0,
            end_minute: 24.0 * 60.0,
        },
    ];

    phases
        .into_iter()
        .find(|phase| minute >= phase.start_minute && minute < phase.end_minute)
        .unwrap_or(phases[0])
}

fn route_sample(
    route: &[(u8, u8); 3],
    route_len: usize,
    progress: f32,
) -> (crate::planet::SurfaceSample, usize) {
    let coordinates: Vec<_> = route[..route_len]
        .iter()
        .map(|&(x, y)| {
            let tile = TileCoordinate::new(PlanetFace::PositiveY, x, y)
                .expect("authored NPC route tiles remain within the planet face");
            let coordinate = PlanetCoordinate::new(tile, FaceOrientation::North);
            let tile = PlanetTile::new(coordinate.tile);
            (
                coordinate,
                sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
                    .expect("the configured planet radius is valid"),
            )
        })
        .collect();
    if coordinates.len() < 2 {
        return (coordinates[0].1, 0);
    }
    let scaled = progress.clamp(0.0, 1.0) * (coordinates.len() - 1) as f32;
    let index = (scaled.floor() as usize).min(coordinates.len() - 2);
    let fraction = scaled - index as f32;
    let (_, start_sample) = coordinates[index];
    let (_, end_sample) = coordinates[index + 1];
    let normal = start_sample
        .normal
        .lerp(end_sample.normal, fraction)
        .normalize();
    let height = coordinates[index]
        .1
        .height
        .lerp(coordinates[index + 1].1.height, fraction);
    (
        crate::planet::SurfaceSample {
            normal,
            height,
            position: normal * (DEFAULT_PLANET_RADIUS + height),
        },
        index + usize::from(fraction > 0.5),
    )
}

fn update_npc_schedules(
    clock: Res<GameClock>,
    mut npcs: Query<(&NpcCharacter, &mut Transform, &mut NpcScheduleDebug)>,
) {
    let minute = clock.minute_of_day();
    for (character, mut transform, mut debug) in &mut npcs {
        let phase = schedule_phase(*character, minute);
        let (sample, waypoint_index) = if phase.route_len == 0 {
            let tile = match phase.destination {
                NpcDestination::Home => match character {
                    NpcCharacter::Baker => (8, 12),
                    NpcCharacter::Carpenter => (11, 12),
                    NpcCharacter::Merchant => (14, 12),
                },
                NpcDestination::Work => match character {
                    NpcCharacter::Baker => (9, 12),
                    NpcCharacter::Carpenter => (12, 12),
                    NpcCharacter::Merchant => (15, 12),
                },
                NpcDestination::Social => SOCIAL_TILE,
            };
            route_sample(&[tile, tile, tile], 1, 0.0)
        } else {
            let progress =
                ((minute - phase.start_minute) / (phase.end_minute - phase.start_minute)) as f32;
            route_sample(&phase.route, phase.route_len, progress)
        };
        let direction = sample.normal;
        if let Some(next_transform) = surface_transform(
            SurfaceLocation::new(direction, 0.04),
            Vec3::ZERO,
            DEFAULT_PLANET_RADIUS,
            &crate::planet::SurfaceSample {
                height: sample.height,
                normal: direction,
                position: sample.position,
            },
            Vec3::X,
        ) {
            *transform = next_transform;
        }
        *debug = NpcScheduleDebug {
            state: phase.state,
            destination: phase.destination,
            waypoint_index,
        };
    }
}

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
                NpcScheduleDebug {
                    state: NpcScheduleState::Home,
                    destination: NpcDestination::Home,
                    waypoint_index: 0,
                },
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
    fn schedule_boundaries_are_deterministic_and_expose_expected_destinations() {
        let cases = [
            (0.0, NpcScheduleState::Home, NpcDestination::Home),
            (8.0 * 60.0, NpcScheduleState::Travel, NpcDestination::Work),
            (9.0 * 60.0, NpcScheduleState::Work, NpcDestination::Work),
            (
                12.0 * 60.0,
                NpcScheduleState::Travel,
                NpcDestination::Social,
            ),
            (
                12.5 * 60.0,
                NpcScheduleState::Social,
                NpcDestination::Social,
            ),
            (13.5 * 60.0, NpcScheduleState::Travel, NpcDestination::Work),
            (14.0 * 60.0, NpcScheduleState::Work, NpcDestination::Work),
            (
                18.0 * 60.0,
                NpcScheduleState::Travel,
                NpcDestination::Social,
            ),
            (
                18.5 * 60.0,
                NpcScheduleState::Social,
                NpcDestination::Social,
            ),
            (19.5 * 60.0, NpcScheduleState::Travel, NpcDestination::Home),
            (21.0 * 60.0, NpcScheduleState::Home, NpcDestination::Home),
        ];
        for character in [
            NpcCharacter::Baker,
            NpcCharacter::Carpenter,
            NpcCharacter::Merchant,
        ] {
            for (minute, expected_state, expected_destination) in cases {
                let schedule = schedule_phase(character, minute);
                assert_eq!(schedule.state, expected_state);
                assert_eq!(schedule.destination, expected_destination);
                assert_eq!(schedule_phase(character, minute).state, schedule.state);
            }
        }
    }

    #[test]
    fn authored_travel_waypoints_interpolate_on_sphere_and_cube_seam() {
        let character = NpcCharacter::Baker;
        let phase = schedule_phase(character, 8.0 * 60.0);
        let (start, _) = route_sample(&phase.route, phase.route_len, 0.0);
        let (middle, _) = route_sample(&phase.route, phase.route_len, 0.5);
        let (end, _) = route_sample(&phase.route, phase.route_len, 1.0);
        assert!((start.position.length() - DEFAULT_PLANET_RADIUS).abs() < 0.1);
        assert!((end.position.length() - DEFAULT_PLANET_RADIUS).abs() < 0.1);
        assert!(middle.normal.distance(start.normal) > 0.0);

        let edge = TileCoordinate::new(PlanetFace::PositiveY, 23, 12).unwrap();
        let across = crate::planet::move_coordinate(
            PlanetCoordinate::new(edge, FaceOrientation::East),
            crate::planet::Direction::East,
        );
        assert_ne!(across.tile.face(), edge.face());
        let edge_sample =
            sample_tile_surface(&PlanetTile::new(edge), DEFAULT_PLANET_RADIUS).unwrap();
        let across_sample =
            sample_tile_surface(&PlanetTile::new(across.tile), DEFAULT_PLANET_RADIUS).unwrap();
        assert!(edge_sample.normal.distance(across_sample.normal) < 0.2);
    }

    #[test]
    fn headless_world_contains_exactly_three_interactable_surface_aligned_npcs_and_properties() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GameClock::default());
        app.add_plugins(NpcPlugin);
        app.update();

        let world = app.world_mut();
        let mut characters = world.query::<(
            &Name,
            &NpcCharacter,
            &Transform,
            &Interactable,
            &NpcScheduleDebug,
        )>();
        let characters: Vec<_> = characters.iter(world).collect();
        assert_eq!(characters.len(), 3);
        for expected in ["Baker", "Carpenter", "Merchant"] {
            assert_eq!(
                characters
                    .iter()
                    .filter(|(name, _, _, _, _)| name.as_str() == expected)
                    .count(),
                1
            );
        }
        for (_, _, transform, interactable, schedule) in characters {
            assert!(interactable.range > 0.0);
            assert_eq!(schedule.state, NpcScheduleState::Home);
            assert_eq!(schedule.destination, NpcDestination::Home);
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
