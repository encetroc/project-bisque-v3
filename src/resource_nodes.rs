//! Deterministic biome-authored resource nodes placed on the spherical surface.

use bevy::prelude::*;

use crate::camera_follow::CameraObstructionFade;
use crate::surface_transform::{SurfaceLocation, surface_transform};
use crate::{
    game_clock::DayTransition,
    interaction::{Interactable, InteractionRequested},
    inventory::Inventory,
    planet::{
        Biome, DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, ResourceType, TILES_PER_FACE,
        TileCoordinate, authored_planet_tile, sample_tile_surface,
    },
};

const OBSTRUCTION_FADE_ALPHA: f32 = 0.3;

/// Stable authored description of a single resource node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceNodeSpawn {
    pub coordinate: TileCoordinate,
    pub biome: Biome,
    pub resource_type: ResourceType,
    pub position: Vec3,
    pub normal: Vec3,
}

/// Marks a world entity as an authored resource node.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceNode {
    pub coordinate: TileCoordinate,
    pub biome: Biome,
    pub resource_type: ResourceType,
}

/// Persistent progression flag set when the player first gathers Highlands red clay.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RedClayDiscovery {
    pub discovered: bool,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GatheredResourceNode;

/// Persistent red-clay progression marker retained for save compatibility.
#[derive(Component, Debug, Clone, Copy)]
pub struct GatheredRedClay;

/// Install deterministic resource-node placement and primitive visuals.
pub struct ResourceNodePlugin;

impl Plugin for ResourceNodePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RedClayDiscovery>()
            .init_resource::<Inventory>()
            .add_message::<InteractionRequested>()
            .add_message::<DayTransition>()
            .add_systems(Startup, spawn_resource_nodes)
            .add_systems(
                Update,
                (gather_resource_nodes, respawn_resource_nodes).chain(),
            );
    }
}

/// Generate a stable sparse scatter of resource nodes from the authored tiles.
///
/// Placement and resource type depend only on a tile's face and grid address,
/// so the same world always produces the same nodes without a random resource.
pub fn authored_resource_node_spawns() -> Vec<ResourceNodeSpawn> {
    let mut spawns = Vec::new();
    for face in PlanetFace::ALL {
        for y in 0..TILES_PER_FACE {
            for x in 0..TILES_PER_FACE {
                let coordinate = TileCoordinate::new(face, x, y)
                    .expect("iteration stays within the authored tile grid");
                let tile = authored_planet_tile(coordinate);
                if tile.deep_water {
                    continue;
                }
                let Some(resource_type) = resource_type_for_tile(&tile) else {
                    continue;
                };
                let sample = sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
                    .expect("the authored planet radius is valid");
                spawns.push(ResourceNodeSpawn {
                    coordinate,
                    biome: tile.biome,
                    resource_type,
                    position: sample.position,
                    normal: sample.normal,
                });
            }
        }
    }
    spawns
}

fn resource_type_for_tile(tile: &PlanetTile) -> Option<ResourceType> {
    // Keep the scatter sparse while making its location independent of runtime
    // iteration order or random-number-generator state.
    let hash = tile_hash(tile.coordinate);
    if !hash.is_multiple_of(13) {
        return None;
    }
    let variety = (hash / 13) % 100;
    Some(match tile.biome {
        Biome::Meadow => match variety % 3 {
            0 => ResourceType::CommonClay,
            1 => ResourceType::Wood,
            _ => ResourceType::Plant,
        },
        Biome::RedHighlands => {
            if variety.is_multiple_of(2) {
                ResourceType::RedClay
            } else {
                ResourceType::IronMineral
            }
        }
        Biome::Coast => match variety % 3 {
            0 => ResourceType::PaleClay,
            1 => ResourceType::Shell,
            _ => ResourceType::Sand,
        },
    })
}

fn tile_hash(coordinate: TileCoordinate) -> u32 {
    let face = PlanetFace::ALL
        .iter()
        .position(|face| *face == coordinate.face())
        .expect("coordinate face is one of the six planet faces") as u32;
    let x = u32::from(coordinate.x());
    let y = u32::from(coordinate.y());
    face * 1_009 + x * 73 + y * 151 + x * y * 17
}

fn spawn_resource_nodes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let visuals = ResourceNodeVisuals::new(&mut meshes, &mut materials);
    for spawn in authored_resource_node_spawns() {
        let tile = authored_planet_tile(spawn.coordinate);
        let sample = sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
            .expect("the authored planet radius is valid");
        let forward_reference = if spawn.normal.dot(Vec3::X).abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Z
        };
        let transform = surface_transform(
            SurfaceLocation::new(spawn.normal, 0.02),
            Vec3::ZERO,
            DEFAULT_PLANET_RADIUS,
            &sample,
            forward_reference,
        )
        .expect("a valid surface normal and tangent reference define an orientation");
        let node = ResourceNode {
            coordinate: spawn.coordinate,
            biome: spawn.biome,
            resource_type: spawn.resource_type,
        };
        let mut entity = commands.spawn((
            Name::new(format!("{:?} resource node", spawn.resource_type)),
            node,
            transform,
        ));
        entity.insert(Interactable::new(gather_prompt(spawn.resource_type)));

        entity.with_children(|children| visuals.spawn(children, spawn.resource_type));
    }
}

fn gather_resource_nodes(
    mut commands: Commands,
    mut requests: MessageReader<InteractionRequested>,
    nodes: Query<&ResourceNode, Without<GatheredResourceNode>>,
    mut inventory: ResMut<Inventory>,
    mut discovery: ResMut<RedClayDiscovery>,
) {
    for request in requests.read() {
        let Ok(node) = nodes.get(request.target) else {
            continue;
        };
        // Each authored node yields one unit of its specified resource. Leave it
        // available if the inventory cannot accept the complete yield.
        if inventory.add_resource(node.resource_type, 1) != 0 {
            continue;
        }
        if node.biome == Biome::RedHighlands && node.resource_type == ResourceType::RedClay {
            discovery.discovered = true;
            commands.entity(request.target).insert(GatheredRedClay);
        }
        commands
            .entity(request.target)
            .insert((GatheredResourceNode, Visibility::Hidden))
            .remove::<Interactable>();
    }
}

fn respawn_resource_nodes(
    mut commands: Commands,
    mut transitions: MessageReader<DayTransition>,
    gathered: Query<(Entity, &ResourceNode), With<GatheredResourceNode>>,
) {
    if transitions.read().next().is_none() {
        return;
    }
    for (entity, node) in &gathered {
        commands
            .entity(entity)
            .remove::<(GatheredResourceNode, GatheredRedClay)>()
            .insert((
                Visibility::Inherited,
                Interactable::new(gather_prompt(node.resource_type)),
            ));
    }
}

pub(crate) fn gather_prompt(resource_type: ResourceType) -> &'static str {
    match resource_type {
        ResourceType::CommonClay => "Gather common clay",
        ResourceType::RedClay => "Gather red clay",
        ResourceType::PaleClay => "Gather pale clay",
        ResourceType::Wood => "Gather wood",
        ResourceType::IronMineral => "Gather iron",
        ResourceType::Plant => "Gather plants",
        ResourceType::Shell => "Gather shells",
        ResourceType::Sand => "Gather sand",
    }
}

struct ResourceNodeVisuals {
    clay_mesh: Handle<Mesh>,
    sphere_mesh: Handle<Mesh>,
    trunk_mesh: Handle<Mesh>,
    mineral_mesh: Handle<Mesh>,
    plant_mesh: Handle<Mesh>,
    clay: Handle<StandardMaterial>,
    red_clay: Handle<StandardMaterial>,
    pale_clay: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    leaves: Handle<StandardMaterial>,
    faded_wood: Handle<StandardMaterial>,
    faded_leaves: Handle<StandardMaterial>,
    mineral: Handle<StandardMaterial>,
    plant: Handle<StandardMaterial>,
    shell: Handle<StandardMaterial>,
    sand: Handle<StandardMaterial>,
}

impl ResourceNodeVisuals {
    fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            clay_mesh: meshes.add(Sphere::new(0.38).mesh().ico(1).unwrap()),
            sphere_mesh: meshes.add(Sphere::new(0.3).mesh().ico(1).unwrap()),
            trunk_mesh: meshes.add(Cylinder::new(0.16, 0.9)),
            mineral_mesh: meshes.add(Cuboid::new(0.38, 0.38, 0.38)),
            plant_mesh: meshes.add(Capsule3d::new(0.11, 0.55)),
            clay: materials.add(Color::srgb(0.48, 0.25, 0.13)),
            red_clay: materials.add(Color::srgb(0.68, 0.20, 0.12)),
            pale_clay: materials.add(Color::srgb(0.78, 0.68, 0.48)),
            wood: materials.add(Color::srgb(0.36, 0.20, 0.10)),
            leaves: materials.add(Color::srgb(0.19, 0.48, 0.18)),
            faded_wood: materials.add(StandardMaterial {
                base_color: Color::srgba(0.36, 0.20, 0.10, OBSTRUCTION_FADE_ALPHA),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            faded_leaves: materials.add(StandardMaterial {
                base_color: Color::srgba(0.19, 0.48, 0.18, OBSTRUCTION_FADE_ALPHA),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            mineral: materials.add(Color::srgb(0.40, 0.43, 0.46)),
            plant: materials.add(Color::srgb(0.25, 0.62, 0.24)),
            shell: materials.add(Color::srgb(0.92, 0.76, 0.61)),
            sand: materials.add(Color::srgb(0.82, 0.72, 0.46)),
        }
    }

    fn spawn(&self, children: &mut ChildSpawnerCommands, resource_type: ResourceType) {
        match resource_type {
            ResourceType::CommonClay => self.spawn_clay(children, &self.clay),
            ResourceType::RedClay => self.spawn_clay(children, &self.red_clay),
            ResourceType::PaleClay => self.spawn_clay(children, &self.pale_clay),
            ResourceType::Wood => {
                self.part_fadeable(
                    children,
                    &self.trunk_mesh,
                    &self.wood,
                    &self.faded_wood,
                    Vec3::Y * 0.46,
                    Vec3::ONE,
                    0.55,
                );
                self.part_fadeable(
                    children,
                    &self.sphere_mesh,
                    &self.leaves,
                    &self.faded_leaves,
                    Vec3::Y * 1.05,
                    Vec3::splat(1.8),
                    1.0,
                );
            }
            ResourceType::IronMineral => {
                for (position, scale) in [
                    (Vec3::new(-0.22, 0.19, 0.0), Vec3::splat(1.0)),
                    (Vec3::new(0.18, 0.23, 0.08), Vec3::splat(0.8)),
                    (Vec3::new(0.0, 0.48, -0.12), Vec3::splat(0.7)),
                ] {
                    self.part(children, &self.mineral_mesh, &self.mineral, position, scale);
                }
            }
            ResourceType::Plant => {
                for (position, rotation) in [
                    (Vec3::new(-0.18, 0.28, 0.0), -0.3),
                    (Vec3::new(0.15, 0.3, 0.05), 0.35),
                    (Vec3::new(0.0, 0.33, -0.12), 0.0),
                ] {
                    self.part_rotated(
                        children,
                        &self.plant_mesh,
                        &self.plant,
                        position,
                        Vec3::splat(0.8),
                        Quat::from_rotation_z(rotation),
                    );
                }
            }
            ResourceType::Shell => self.spawn_cluster(children, &self.sphere_mesh, &self.shell),
            ResourceType::Sand => self.spawn_cluster(children, &self.sphere_mesh, &self.sand),
        }
    }

    fn spawn_clay(&self, children: &mut ChildSpawnerCommands, material: &Handle<StandardMaterial>) {
        self.spawn_cluster(children, &self.clay_mesh, material);
    }

    fn spawn_cluster(
        &self,
        children: &mut ChildSpawnerCommands,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
    ) {
        for (position, scale) in [
            (Vec3::new(-0.23, 0.22, 0.0), Vec3::splat(0.82)),
            (Vec3::new(0.2, 0.25, 0.06), Vec3::splat(0.72)),
            (Vec3::new(0.0, 0.45, -0.12), Vec3::splat(0.67)),
        ] {
            self.part(children, mesh, material, position, scale);
        }
    }

    fn part_fadeable(
        &self,
        children: &mut ChildSpawnerCommands,
        mesh: &Handle<Mesh>,
        opaque_material: &Handle<StandardMaterial>,
        faded_material: &Handle<StandardMaterial>,
        translation: Vec3,
        scale: Vec3,
        radius: f32,
    ) {
        children.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(opaque_material.clone()),
            CameraObstructionFade {
                opaque_material: opaque_material.clone(),
                faded_material: faded_material.clone(),
                radius,
            },
            Transform {
                translation,
                scale,
                ..default()
            },
        ));
    }

    fn part(
        &self,
        children: &mut ChildSpawnerCommands,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
        translation: Vec3,
        scale: Vec3,
    ) {
        self.part_rotated(children, mesh, material, translation, scale, Quat::IDENTITY);
    }

    fn part_rotated(
        &self,
        children: &mut ChildSpawnerCommands,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
        translation: Vec3,
        scale: Vec3,
        rotation: Quat,
    ) {
        children.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform {
                translation,
                rotation,
                scale,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::InventorySlot;
    use std::collections::HashSet;

    #[test]
    fn resource_spawns_are_deterministic_biome_compatible_and_never_in_deep_water() {
        let spawns = authored_resource_node_spawns();
        assert_eq!(spawns, authored_resource_node_spawns());
        assert!(
            spawns.len() > 100,
            "the planet should have a discoverable scatter"
        );

        let mut meadow = 0;
        let mut highlands = 0;
        let mut coast = 0;
        let mut biome_resources = HashSet::new();
        for spawn in spawns {
            let tile = authored_planet_tile(spawn.coordinate);
            assert!(!tile.deep_water);
            assert_eq!(tile.biome, spawn.biome);
            let compatible = match spawn.biome {
                Biome::Meadow => {
                    meadow += 1;
                    matches!(
                        spawn.resource_type,
                        ResourceType::CommonClay | ResourceType::Wood | ResourceType::Plant
                    )
                }
                Biome::RedHighlands => {
                    highlands += 1;
                    matches!(
                        spawn.resource_type,
                        ResourceType::RedClay | ResourceType::IronMineral
                    )
                }
                Biome::Coast => {
                    coast += 1;
                    matches!(
                        spawn.resource_type,
                        ResourceType::PaleClay | ResourceType::Shell | ResourceType::Sand
                    )
                }
            };
            assert!(compatible, "{spawn:?} is incompatible with its biome");
            biome_resources.insert((spawn.biome, spawn.resource_type));
            assert!(spawn.position.is_finite());
            assert!((spawn.position.length() - DEFAULT_PLANET_RADIUS).abs() < 1e-4);
            assert!((spawn.normal.length() - 1.0).abs() < 1e-5);
            assert!(spawn.position.normalize().dot(spawn.normal) > 0.99999);
        }
        assert!(meadow > 0 && highlands > 0 && coast > 0);
        for (biome, resource_type) in [
            (Biome::Meadow, ResourceType::CommonClay),
            (Biome::Meadow, ResourceType::Wood),
            (Biome::Meadow, ResourceType::Plant),
            (Biome::RedHighlands, ResourceType::RedClay),
            (Biome::RedHighlands, ResourceType::IronMineral),
            (Biome::Coast, ResourceType::PaleClay),
            (Biome::Coast, ResourceType::Shell),
            (Biome::Coast, ResourceType::Sand),
        ] {
            assert!(
                biome_resources.contains(&(biome, resource_type)),
                "missing {resource_type:?} in {biome:?}"
            );
        }
    }

    #[test]
    fn red_clay_discovery_is_highlands_only_while_other_nodes_can_be_gathered() {
        let mut app = App::new();
        app.add_message::<InteractionRequested>()
            .init_resource::<Inventory>()
            .init_resource::<RedClayDiscovery>()
            .add_systems(Update, gather_resource_nodes);
        let highlands = app
            .world_mut()
            .spawn(ResourceNode {
                coordinate: TileCoordinate::new(PlanetFace::PositiveX, 1, 1).unwrap(),
                biome: Biome::RedHighlands,
                resource_type: ResourceType::RedClay,
            })
            .id();
        let invalid_meadow = app
            .world_mut()
            .spawn(ResourceNode {
                coordinate: TileCoordinate::new(PlanetFace::PositiveY, 1, 1).unwrap(),
                biome: Biome::Meadow,
                resource_type: ResourceType::RedClay,
            })
            .id();

        app.world_mut().write_message(InteractionRequested {
            target: invalid_meadow,
        });
        app.update();
        assert!(!app.world().resource::<RedClayDiscovery>().discovered);
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::RedClay),
            1
        );

        app.world_mut()
            .write_message(InteractionRequested { target: highlands });
        app.update();
        assert!(app.world().resource::<RedClayDiscovery>().discovered);
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::RedClay),
            2
        );
        assert!(app.world().get::<GatheredResourceNode>(highlands).is_some());
        assert!(app.world().get::<GatheredRedClay>(highlands).is_some());
        assert_eq!(
            app.world().get::<Visibility>(highlands),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().get::<Interactable>(highlands).is_none());
        assert!(app.world().get::<ResourceNode>(highlands).is_some());

        app.world_mut()
            .write_message(InteractionRequested { target: highlands });
        app.update();
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::RedClay),
            2,
            "a previously gathered node cannot be collected again before respawning"
        );
    }

    #[test]
    fn gathering_any_node_hides_it_and_only_a_day_transition_respawns_it() {
        let mut app = App::new();
        app.add_message::<InteractionRequested>()
            .add_message::<DayTransition>()
            .init_resource::<Inventory>()
            .init_resource::<RedClayDiscovery>()
            .add_systems(
                Update,
                (gather_resource_nodes, respawn_resource_nodes).chain(),
            );
        let node = app
            .world_mut()
            .spawn((
                ResourceNode {
                    coordinate: TileCoordinate::new(PlanetFace::PositiveX, 2, 2).unwrap(),
                    biome: Biome::Meadow,
                    resource_type: ResourceType::Wood,
                },
                Interactable::new("Gather wood"),
                Visibility::Inherited,
            ))
            .id();

        app.world_mut()
            .write_message(InteractionRequested { target: node });
        app.update();
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::Wood),
            1
        );
        assert!(app.world().get::<GatheredResourceNode>(node).is_some());
        assert_eq!(
            app.world().get::<Visibility>(node),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().get::<Interactable>(node).is_none());

        app.update();
        assert!(app.world().get::<GatheredResourceNode>(node).is_some());
        app.world_mut().write_message(DayTransition { day: 2 });
        app.update();
        assert!(app.world().get::<GatheredResourceNode>(node).is_none());
        assert_eq!(
            app.world().get::<Visibility>(node),
            Some(&Visibility::Inherited)
        );
        assert!(app.world().get::<Interactable>(node).is_some());
    }

    #[test]
    fn full_inventory_leaves_node_visible_and_yield_uncollected() {
        let mut app = App::new();
        app.add_message::<InteractionRequested>()
            .init_resource::<Inventory>()
            .init_resource::<RedClayDiscovery>()
            .add_systems(Update, gather_resource_nodes);
        let node = app
            .world_mut()
            .spawn((
                ResourceNode {
                    coordinate: TileCoordinate::new(PlanetFace::PositiveX, 3, 3).unwrap(),
                    biome: Biome::Meadow,
                    resource_type: ResourceType::Wood,
                },
                Interactable::new("Gather wood"),
                Visibility::Inherited,
            ))
            .id();
        let slots = std::array::from_fn(|index| {
            InventorySlot::Ceramic(crate::inventory::CeramicObjectId(index as u64))
        });
        app.world_mut()
            .resource_mut::<Inventory>()
            .restore_slots(slots);

        app.world_mut()
            .write_message(InteractionRequested { target: node });
        app.update();

        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::Wood),
            0
        );
        assert!(app.world().get::<GatheredResourceNode>(node).is_none());
        assert_eq!(
            app.world().get::<Visibility>(node),
            Some(&Visibility::Inherited)
        );
        assert!(app.world().get::<Interactable>(node).is_some());
    }

    #[test]
    fn headless_plugin_spawns_surface_aligned_nodes_with_primitive_children() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.add_plugins(ResourceNodePlugin);
        app.update();

        let world = app.world_mut();
        let mut nodes = world.query::<(&ResourceNode, &Transform, &Children)>();
        let node_data: Vec<_> = nodes
            .iter(world)
            .map(|(node, transform, children)| (*node, *transform, children.len()))
            .collect();
        assert_eq!(node_data.len(), authored_resource_node_spawns().len());
        assert!(node_data.iter().all(|(_, transform, child_count)| {
            *child_count >= 2
                && transform.translation.is_finite()
                && transform.rotation.is_finite()
                && transform
                    .rotation
                    .mul_vec3(Vec3::Y)
                    .dot(transform.translation.normalize())
                    > 0.999
        }));

        let fadeable_data = {
            let mut fadeables = world.query::<&CameraObstructionFade>();
            fadeables
                .iter(world)
                .map(|fadeable| (fadeable.faded_material.clone(), fadeable.radius))
                .collect::<Vec<_>>()
        };
        assert!(!fadeable_data.is_empty());
        let materials = world.resource::<Assets<StandardMaterial>>();
        assert!(fadeable_data.iter().all(|(handle, radius)| {
            let material = materials.get(handle).unwrap();
            *radius > 0.0
                && material.alpha_mode == AlphaMode::Blend
                && (material.base_color.alpha() - OBSTRUCTION_FADE_ALPHA).abs() < f32::EPSILON
        }));
    }
}
