//! Primitive, deterministic visuals for ceramic items.

use bevy::prelude::*;

use crate::{
    ceramics::{
        CERAMIC_RECIPES, CeramicForm, CeramicItem, CeramicItemTemplate, ClayMaterial, Glaze,
        ProcessingState,
    },
    planet::{DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface},
    surface_transform::{SurfaceLocation, surface_transform},
};

const CUP_RADIUS: f32 = 0.28;
const CUP_HEIGHT: f32 = 0.52;

/// Marks the root entity of a ceramic representation, retaining its source data.
#[derive(Component, Debug, Clone, Copy)]
pub struct CeramicVisual(pub CeramicItem);

/// Installs the visual catalog display alongside the planet test scene.
pub struct CeramicVisualPlugin;

impl Plugin for CeramicVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ceramic_visual_catalog);
    }
}

/// Number of primitive meshes used to make a form recognizable.
pub fn ceramic_primitive_count(form: CeramicForm) -> usize {
    match form {
        CeramicForm::Cup | CeramicForm::Bowl => 2,
        CeramicForm::Vase => 3,
    }
}

/// Clay/glaze tint for an item. Unfired stages are intentionally less saturated;
/// fired items retain vivid glaze colors. This mapping is shared by every visual.
pub fn ceramic_color(template: CeramicItemTemplate) -> Color {
    let clay = match template.clay {
        ClayMaterial::Common => Color::srgb(0.72, 0.48, 0.31),
        ClayMaterial::Red => Color::srgb(0.67, 0.25, 0.16),
        ClayMaterial::Pale => Color::srgb(0.88, 0.78, 0.63),
    };
    let glaze = match template.glaze {
        Glaze::None => clay,
        Glaze::Blue => Color::srgb(0.12, 0.38, 0.78),
        Glaze::Green => Color::srgb(0.18, 0.58, 0.35),
        Glaze::White => Color::srgb(0.94, 0.91, 0.82),
    };
    let blend = if template.glaze == Glaze::None {
        0.0
    } else {
        0.78
    };
    let color = clay.mix(&glaze, blend);
    match template.state {
        ProcessingState::Greenware => color.mix(&Color::srgb(0.64, 0.53, 0.43), 0.48),
        ProcessingState::Dry => color.mix(&Color::srgb(0.82, 0.72, 0.58), 0.24),
        ProcessingState::Fired => color,
    }
}

/// Spawn a data-backed ceramic root and its primitive mesh children.
pub fn spawn_ceramic_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: CeramicItem,
    transform: Transform,
) -> Entity {
    spawn_ceramic_visual_with_style(commands, meshes, materials, item, transform, false)
}

/// Spawn a translucent, unlit placement preview using the same item-to-form mapping.
pub fn spawn_ceramic_ghost(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: CeramicItem,
    transform: Transform,
) -> Entity {
    spawn_ceramic_visual_with_style(commands, meshes, materials, item, transform, true)
}

fn spawn_ceramic_visual_with_style(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: CeramicItem,
    transform: Transform,
    ghost: bool,
) -> Entity {
    let mut color = ceramic_color(item.template);
    if ghost {
        color.set_alpha(0.45);
    }
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: if ghost {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: ghost,
        perceptual_roughness: if item.state() == ProcessingState::Fired {
            0.28
        } else {
            0.82
        },
        ..default()
    });
    let root = commands
        .spawn((
            Name::new(format!(
                "Ceramic {:?} {:?} {:?}",
                item.form(),
                item.clay(),
                item.glaze()
            )),
            CeramicVisual(item),
            transform,
            Visibility::default(),
        ))
        .id();
    commands
        .entity(root)
        .with_children(|children| match item.form() {
            CeramicForm::Cup => {
                children.spawn((
                    Mesh3d(meshes.add(Cylinder::new(CUP_RADIUS, CUP_HEIGHT))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, CUP_HEIGHT / 2.0, 0.0),
                ));
                children.spawn((
                    Mesh3d(meshes.add(Torus::new(0.20, 0.26))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, CUP_HEIGHT / 2.0, CUP_RADIUS * 0.92)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
            }
            CeramicForm::Bowl => {
                children.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.42))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.12, 0.0).with_scale(Vec3::new(1.0, 0.52, 1.0)),
                ));
                children.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.18, 0.07))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.035, 0.0),
                ));
            }
            CeramicForm::Vase => {
                children.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.36))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.38, 0.0).with_scale(Vec3::new(0.9, 1.15, 0.9)),
                ));
                children.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.17, 0.38))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.78, 0.0),
                ));
                children.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.22, 0.08))),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.99, 0.0),
                ));
            }
        });
    root
}

fn spawn_ceramic_visual_catalog(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A compact swatch shelf shows every supported form, clay, and glaze combination
    // beside the planet test player's starting location.
    let tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14).expect("catalog tile is valid"),
    );
    let surface =
        sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS).expect("catalog surface sample is valid");
    let mut frame = surface_transform(
        SurfaceLocation::new(surface.normal, 0.1),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("catalog tangent frame is valid");
    frame.translation += frame.rotation * Vec3::X * 5.0;
    let backing = materials.add(Color::srgb(0.25, 0.24, 0.22));
    let backing_mesh = meshes.add(Cuboid::new(11.0, 0.16, 8.0));
    let backing_transform = frame * Transform::from_xyz(0.0, 0.0, 0.0);
    commands.spawn((
        Name::new("Ceramic visual catalog backing"),
        Mesh3d(backing_mesh),
        MeshMaterial3d(backing),
        backing_transform,
    ));

    for (index, recipe) in CERAMIC_RECIPES.iter().enumerate() {
        let item = recipe
            .output
            .instantiate(crate::inventory::CeramicObjectId(index as u64 + 1));
        let column = index % 9;
        let row = index / 9;
        let local = Transform::from_xyz(-4.0 + column as f32, 0.15, -2.5 + row as f32 * 1.7);
        spawn_ceramic_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            item,
            frame * local,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_item_has_a_stable_visual_mapping() {
        assert_eq!(CERAMIC_RECIPES.len(), 36);
        for (index, recipe) in CERAMIC_RECIPES.iter().enumerate() {
            let item = recipe
                .output
                .instantiate(crate::inventory::CeramicObjectId(index as u64));
            assert_eq!(item.form(), recipe.output.form);
            assert_ne!(ceramic_color(item.template), Color::BLACK);
        }
        for form in CeramicForm::ALL {
            for clay in ClayMaterial::ALL {
                for glaze in Glaze::ALL {
                    let template = CERAMIC_RECIPES
                        .iter()
                        .find(|recipe| {
                            recipe.output.form == form
                                && recipe.output.clay == clay
                                && recipe.output.glaze == glaze
                        })
                        .unwrap()
                        .output;
                    assert_ne!(ceramic_color(template), Color::BLACK);
                }
            }
        }
    }

    #[test]
    fn catalog_plugin_spawns_every_visual_without_a_window() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(CeramicVisualPlugin);
        app.update();

        let mut visuals = app.world_mut().query::<&CeramicVisual>();
        assert_eq!(visuals.iter(app.world()).count(), CERAMIC_RECIPES.len());
        let mut meshes = app.world_mut().query::<&Mesh3d>();
        assert_eq!(
            meshes.iter(app.world()).count(),
            1 + 2 * 12 + 2 * 12 + 3 * 12
        );
    }

    #[test]
    fn clay_and_glaze_variations_are_visible_and_form_shapes_are_distinct() {
        let template = CeramicItemTemplate {
            form: CeramicForm::Cup,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state: ProcessingState::Fired,
        };
        let common = ceramic_color(template);
        assert_ne!(
            common,
            ceramic_color(CeramicItemTemplate {
                clay: ClayMaterial::Red,
                ..template
            })
        );
        assert_ne!(
            common,
            ceramic_color(CeramicItemTemplate {
                glaze: Glaze::Blue,
                ..template
            })
        );
        assert_ne!(
            common,
            ceramic_color(CeramicItemTemplate {
                glaze: Glaze::Green,
                ..template
            })
        );
        assert_ne!(
            common,
            ceramic_color(CeramicItemTemplate {
                glaze: Glaze::White,
                ..template
            })
        );
        assert_eq!(ceramic_primitive_count(CeramicForm::Cup), 2);
        assert_eq!(ceramic_primitive_count(CeramicForm::Bowl), 2);
        assert_eq!(ceramic_primitive_count(CeramicForm::Vase), 3);
        assert_eq!(CUP_RADIUS, 0.28);
        assert_eq!(CUP_HEIGHT, 0.52);
    }
}
