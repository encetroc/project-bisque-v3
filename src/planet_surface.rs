//! Low-resolution rendered surface and geometry-debug scene for the six logical planet faces.

use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

use crate::planet::{
    DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TILES_PER_FACE, TileCoordinate,
    project_face_to_sphere, sample_tile_surface,
};
use crate::player_movement::SurfacePlayer;
use crate::surface_transform::{SurfaceLocation, surface_transform};

/// Number of logical tiles across the complete planet.
pub const PLANET_TILE_COUNT: usize =
    PlanetFace::ALL.len() * TILES_PER_FACE as usize * TILES_PER_FACE as usize;

const FACE_DEBUG_COLORS: [Color; 6] = [
    Color::srgb(0.92, 0.20, 0.18),
    Color::srgb(0.18, 0.46, 0.95),
    Color::srgb(0.25, 0.82, 0.34),
    Color::srgb(0.95, 0.74, 0.16),
    Color::srgb(0.72, 0.28, 0.88),
    Color::srgb(0.16, 0.82, 0.82),
];
const SURFACE_COLOR: Color = Color::srgb(0.34, 0.58, 0.30);
const TILE_LINE_COLOR: Color = Color::srgb(0.025, 0.025, 0.025);
const NORMAL_LINE_COLOR: Color = Color::srgb(1.0, 0.95, 0.25);

/// CPU-side mesh data with face identity retained for inspection and tests.
#[derive(Debug, Clone)]
pub struct PlanetFaceMesh {
    pub face: PlanetFace,
    pub tiles: Vec<PlanetTile>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl PlanetFaceMesh {
    /// Build one face as 24×24 tile quads (25×25 projected vertices).
    pub fn new(face: PlanetFace, radius: f32) -> Self {
        let edge = TILES_PER_FACE as usize;
        let mut positions = Vec::with_capacity((edge + 1) * (edge + 1));
        let mut normals = Vec::with_capacity((edge + 1) * (edge + 1));
        for y in 0..=edge {
            for x in 0..=edge {
                let u = x as f32 / edge as f32 * 2.0 - 1.0;
                let v = y as f32 / edge as f32 * 2.0 - 1.0;
                let point = project_face_to_sphere(face, u, v, radius)
                    .expect("grid points and configured planet radius are valid");
                positions.push(point.to_array());
                normals.push(point.normalize().to_array());
            }
        }

        let mut tiles = Vec::with_capacity(edge * edge);
        let mut indices = Vec::with_capacity(edge * edge * 6);
        for y in 0..edge {
            for x in 0..edge {
                let coordinate = TileCoordinate::new(face, x as u8, y as u8)
                    .expect("mesh grid coordinates are within the face");
                tiles.push(PlanetTile::new(coordinate));
                let a = (y * (edge + 1) + x) as u32;
                let b = a + 1;
                let c = a + (edge + 1) as u32;
                let d = c + 1;
                let pa = Vec3::from_array(positions[a as usize]);
                let pb = Vec3::from_array(positions[b as usize]);
                let pc = Vec3::from_array(positions[c as usize]);
                if (pb - pa).cross(pc - pa).dot(pa + pb + pc) >= 0.0 {
                    indices.extend_from_slice(&[a, b, c, b, d, c]);
                } else {
                    indices.extend_from_slice(&[a, c, b, b, c, d]);
                }
            }
        }

        Self {
            face,
            tiles,
            positions,
            normals,
            indices,
        }
    }

    /// Convert the generated data into a Bevy triangle-list mesh.
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_indices(bevy::mesh::Indices::U32(self.indices));
        mesh
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct PlanetFaceDebugColor(pub Color);

/// Logical tiles retained on the rendered face for gameplay/debug inspection.
#[derive(Component, Debug, Clone)]
pub struct PlanetFaceTiles(pub Vec<PlanetTile>);

/// Current visibility of the three planet geometry diagnostics.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlanetDebugMode {
    pub face_colors: bool,
    pub tile_boundaries: bool,
    pub surface_normals: bool,
}

impl Default for PlanetDebugMode {
    fn default() -> Self {
        Self {
            face_colors: true,
            tile_boundaries: false,
            surface_normals: false,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum PlanetOverlay {
    TileBoundaries,
    SurfaceNormals,
}

/// Installs the geometry-only planet test scene and its surface diagnostics.
pub struct PlanetSurfacePlugin;

impl Plugin for PlanetSurfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlanetDebugMode>()
            .add_systems(Startup, spawn_planet_surface)
            .add_systems(Update, toggle_planet_debug);
    }
}

fn spawn_planet_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mode: Res<PlanetDebugMode>,
) {
    for (index, face) in PlanetFace::ALL.into_iter().enumerate() {
        let debug_color = FACE_DEBUG_COLORS[index];
        let generated = PlanetFaceMesh::new(face, DEFAULT_PLANET_RADIUS);
        let tiles = generated.tiles.clone();
        let mesh = meshes.add(generated.into_mesh());
        let material = materials.add(StandardMaterial {
            base_color: debug_color,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Name::new(format!("Planet face {face:?}")),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            PlanetFaceDebugColor(debug_color),
            PlanetFaceTiles(tiles),
        ));

        let boundary_mesh = meshes.add(line_mesh(tile_boundary_lines(
            face,
            DEFAULT_PLANET_RADIUS * 1.001,
        )));
        let boundary_material = materials.add(StandardMaterial {
            base_color: TILE_LINE_COLOR,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Name::new(format!("Planet tile boundaries {face:?}")),
            Mesh3d(boundary_mesh),
            MeshMaterial3d(boundary_material),
            PlanetOverlay::TileBoundaries,
            visibility(mode.tile_boundaries),
        ));

        let normal_mesh = meshes.add(line_mesh(surface_normal_lines(face, DEFAULT_PLANET_RADIUS)));
        let normal_material = materials.add(StandardMaterial {
            base_color: NORMAL_LINE_COLOR,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Name::new(format!("Planet surface normals {face:?}")),
            Mesh3d(normal_mesh),
            MeshMaterial3d(normal_material),
            PlanetOverlay::SurfaceNormals,
            visibility(mode.surface_normals),
        ));
    }

    // The neutral capsule marks a non-equatorial location using the shared
    // surface transform API, just like future actors and placed props.
    let start_tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14)
            .expect("test player tile is within the face"),
    );
    let start_surface = sample_tile_surface(&start_tile, DEFAULT_PLANET_RADIUS)
        .expect("test planet radius is valid");
    let player_transform = surface_transform(
        SurfaceLocation::new(start_surface.normal, 1.6),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &start_surface,
        Vec3::X,
    )
    .expect("test player location and tangent heading are valid");
    commands.spawn((
        Name::new("Planet test player placeholder"),
        Mesh3d(meshes.add(Capsule3d::new(0.65, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.95, 0.82, 0.62))),
        SurfacePlayer::new(
            SurfaceLocation::new(start_surface.normal, 1.6),
            Vec3::X,
            DEFAULT_PLANET_RADIUS,
        ),
        player_transform,
    ));

    commands.spawn((
        Name::new("Planet test controls"),
        Text::new("Planet test  |  WASD: walk  Shift: run  |  Q/E: camera yaw  Wheel: zoom (10–16)  |  F: face colors  T: tile boundaries  N: normals"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn tile_boundary_lines(face: PlanetFace, radius: f32) -> Vec<Vec3> {
    let edge = TILES_PER_FACE as usize;
    let mut vertices = Vec::with_capacity(edge * (edge + 1) * 4);
    for grid in 0..=edge {
        let coord = grid as f32 / edge as f32 * 2.0 - 1.0;
        for step in 0..edge {
            let start = step as f32 / edge as f32 * 2.0 - 1.0;
            let end = (step + 1) as f32 / edge as f32 * 2.0 - 1.0;
            vertices.push(project_face_to_sphere(face, start, coord, radius).unwrap());
            vertices.push(project_face_to_sphere(face, end, coord, radius).unwrap());
            vertices.push(project_face_to_sphere(face, coord, start, radius).unwrap());
            vertices.push(project_face_to_sphere(face, coord, end, radius).unwrap());
        }
    }
    vertices
}

fn surface_normal_lines(face: PlanetFace, radius: f32) -> Vec<Vec3> {
    let edge = TILES_PER_FACE as usize;
    let mut vertices = Vec::with_capacity(edge * edge * 2);
    for y in 0..edge {
        for x in 0..edge {
            let u = (x as f32 + 0.5) / edge as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / edge as f32 * 2.0 - 1.0;
            let normal = project_face_to_sphere(face, u, v, 1.0).unwrap();
            vertices.push(normal * (radius + 0.08));
            vertices.push(normal * (radius + 1.8));
        }
    }
    vertices
}

fn line_mesh(vertices: Vec<Vec3>) -> Mesh {
    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
}

fn visibility(visible: bool) -> Visibility {
    if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn toggle_planet_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlanetDebugMode>,
    faces: Query<(&PlanetFaceDebugColor, &MeshMaterial3d<StandardMaterial>)>,
    mut overlays: Query<(&PlanetOverlay, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        mode.face_colors = !mode.face_colors;
        for (debug, material_handle) in &faces {
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.base_color = if mode.face_colors {
                    debug.0
                } else {
                    SURFACE_COLOR
                };
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyT) {
        mode.tile_boundaries = !mode.tile_boundaries;
        for (overlay, mut visible) in &mut overlays {
            if *overlay == PlanetOverlay::TileBoundaries {
                *visible = visibility(mode.tile_boundaries);
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        mode.surface_normals = !mode.surface_normals;
        for (overlay, mut visible) in &mut overlays {
            if *overlay == PlanetOverlay::SurfaceNormals {
                *visible = visibility(mode.surface_normals);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_has_expected_tile_vertices_and_valid_triangle_indices() {
        let mut tile_total = 0;
        for face in PlanetFace::ALL {
            let generated = PlanetFaceMesh::new(face, DEFAULT_PLANET_RADIUS);
            assert_eq!(generated.face, face);
            assert_eq!(generated.tiles.len(), 24 * 24);
            assert_eq!(generated.positions.len(), 25 * 25);
            assert_eq!(generated.normals.len(), generated.positions.len());
            assert_eq!(generated.indices.len(), 24 * 24 * 6);
            assert!(
                generated
                    .indices
                    .iter()
                    .all(|&index| index < generated.positions.len() as u32)
            );
            assert!(
                generated
                    .tiles
                    .iter()
                    .all(|tile| tile.coordinate.face() == face)
            );
            assert!(generated.positions.iter().all(|position| {
                let position = Vec3::from_array(*position);
                position.is_finite() && (position.length() - DEFAULT_PLANET_RADIUS).abs() < 1e-4
            }));
            tile_total += generated.tiles.len();
        }
        assert_eq!(tile_total, PLANET_TILE_COUNT);
        assert_eq!(tile_total, 3_456);
    }

    #[test]
    fn headless_scene_spawns_all_faces_player_and_debug_overlays() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.init_resource::<PlanetDebugMode>();
        app.add_systems(Startup, spawn_planet_surface);
        app.update();

        let world = app.world_mut();
        let mut faces = world.query::<(&Mesh3d, &PlanetFaceTiles, &PlanetFaceDebugColor)>();
        let spawned: Vec<_> = faces.iter(world).collect();
        assert_eq!(spawned.len(), PlanetFace::ALL.len());
        assert_eq!(
            spawned
                .iter()
                .map(|(_, tiles, _)| tiles.0.len())
                .sum::<usize>(),
            PLANET_TILE_COUNT
        );
        let mut overlays = world.query::<(&PlanetOverlay, &Visibility)>();
        let overlays: Vec<_> = overlays.iter(world).collect();
        assert_eq!(overlays.len(), PlanetFace::ALL.len() * 2);
        assert!(
            overlays
                .iter()
                .all(|(_, visible)| **visible == Visibility::Hidden)
        );
        let mut player = world.query_filtered::<Entity, With<Name>>();
        assert!(player.iter(world).any(|entity| {
            world
                .get::<Name>(entity)
                .is_some_and(|name| name.as_str() == "Planet test player placeholder")
        }));
    }

    #[test]
    fn debug_hotkeys_toggle_face_colors_tile_boundaries_and_normals() {
        let mut app = App::new();
        app.init_resource::<PlanetDebugMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(Assets::<StandardMaterial>::default())
            .add_systems(Update, toggle_planet_debug);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyT);
        app.update();
        assert!(app.world().resource::<PlanetDebugMode>().tile_boundaries);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyN);
        app.update();
        assert!(app.world().resource::<PlanetDebugMode>().surface_normals);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        assert!(!app.world().resource::<PlanetDebugMode>().face_colors);
    }

    #[test]
    fn boundary_and_normal_overlays_are_complete_and_on_the_surface() {
        for face in PlanetFace::ALL {
            let boundaries = tile_boundary_lines(face, DEFAULT_PLANET_RADIUS * 1.001);
            let normals = surface_normal_lines(face, DEFAULT_PLANET_RADIUS);
            assert_eq!(boundaries.len(), 24 * 25 * 4);
            assert!(boundaries.iter().all(|point| {
                point.is_finite() && (point.length() - DEFAULT_PLANET_RADIUS * 1.001).abs() < 1e-4
            }));
            assert_eq!(normals.len(), 24 * 24 * 2);
            assert!(normals.chunks_exact(2).all(|line| {
                let direction = line[1] - line[0];
                direction.length() > 1.7 && direction.normalize().dot(line[0].normalize()) > 0.999
            }));
        }
    }

    #[test]
    fn every_face_mesh_is_outward_wound_and_face_identity_is_distinct() {
        for face in PlanetFace::ALL {
            let generated = PlanetFaceMesh::new(face, DEFAULT_PLANET_RADIUS);
            for triangle in generated.indices.chunks_exact(3) {
                let a = Vec3::from_array(generated.positions[triangle[0] as usize]);
                let b = Vec3::from_array(generated.positions[triangle[1] as usize]);
                let c = Vec3::from_array(generated.positions[triangle[2] as usize]);
                assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
            }
            assert!(
                generated
                    .tiles
                    .iter()
                    .all(|tile| tile.coordinate.face() == face)
            );
        }
    }
}
