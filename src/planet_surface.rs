//! Low-resolution rendered surface for the six logical planet faces.

use bevy::{mesh::Indices, prelude::*};

use crate::planet::{
    DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TILES_PER_FACE, TileCoordinate,
    project_face_to_sphere,
};

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
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct PlanetFaceDebugColor(pub Color);

/// Logical tiles retained on the rendered face for gameplay/debug inspection.
#[derive(Component, Debug, Clone)]
pub struct PlanetFaceTiles(pub Vec<PlanetTile>);

#[derive(Resource, Debug, Clone, Copy)]
pub struct PlanetFaceDebugMode(pub bool);

impl Default for PlanetFaceDebugMode {
    fn default() -> Self {
        Self(true)
    }
}

/// Installs one immediately available planet scene and its face-color toggle.
pub struct PlanetSurfacePlugin;

impl Plugin for PlanetSurfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlanetFaceDebugMode>()
            .add_systems(Startup, spawn_planet_surface)
            .add_systems(Update, toggle_face_debug_colors);
    }
}

fn spawn_planet_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    }

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 65.0, 95.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle_face_debug_colors(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlanetFaceDebugMode>,
    faces: Query<(&PlanetFaceDebugColor, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    mode.0 = !mode.0;
    for (debug, material_handle) in &faces {
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.base_color = if mode.0 { debug.0 } else { SURFACE_COLOR };
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
    fn headless_scene_spawns_all_six_face_meshes_without_loading_states() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
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
    }

    #[test]
    fn each_face_mesh_is_outward_wound_and_face_identity_is_distinct() {
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
