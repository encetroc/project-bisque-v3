//! Surface-relative placement mode for ceramic inventory objects.

use bevy::{input::mouse::MouseButton, prelude::*};

use crate::{
    ceramic_visuals::{spawn_ceramic_ghost, spawn_ceramic_visual},
    ceramics::CeramicItem,
    inventory::{CeramicObjectId, Inventory, InventorySlot},
    planet::DEFAULT_PLANET_RADIUS,
    surface_transform::SurfaceLocation,
    workbench::CraftedCeramics,
};

const OBJECT_CLEARANCE: f32 = 0.08;
const ROTATION_STEP: f32 = std::f32::consts::FRAC_PI_4;

/// Runtime state for the currently selected inventory object and its preview.
#[derive(Resource, Debug, Default)]
pub struct PlacementMode {
    pub active: bool,
    pub selected: Option<CeramicObjectId>,
    pub surface: Option<PlacementSurface>,
    pub rotation: f32,
    ghost: Option<Entity>,
}

/// A valid camera-ray intersection with the planet surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementSurface {
    pub location: SurfaceLocation,
    pub normal: Vec3,
}

/// Persistent-ready data attached to an object placed by the player.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PlayerPlacedObject {
    pub item: CeramicItem,
    pub location: SurfaceLocation,
    pub rotation: f32,
}

#[derive(Component)]
struct PlacementHelpText;

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementMode>()
            .add_systems(Startup, spawn_placement_help)
            .add_systems(Update, placement_controls);
    }
}

/// Find the nearest positive ray/sphere intersection and return its radial up.
pub fn raycast_planet_surface(
    ray_origin: Vec3,
    ray_direction: Vec3,
    radius: f32,
) -> Option<PlacementSurface> {
    if !ray_origin.is_finite() || !ray_direction.is_finite() || !radius.is_finite() || radius <= 0.0
    {
        return None;
    }
    let direction = ray_direction.try_normalize()?;
    let b = ray_origin.dot(direction);
    let c = ray_origin.length_squared() - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 || !discriminant.is_finite() {
        return None;
    }
    let root = discriminant.sqrt();
    let distance = [-b - root, -b + root]
        .into_iter()
        .filter(|distance| *distance > 0.0)
        .min_by(f32::total_cmp)?;
    let point = ray_origin + direction * distance;
    let normal = point.try_normalize()?;
    Some(PlacementSurface {
        location: SurfaceLocation::new(normal, OBJECT_CLEARANCE),
        normal,
    })
}

/// Build an orientation whose local up follows the hit normal and whose forward
/// is rotated around that normal by the placement yaw.
pub fn placement_rotation(normal: Vec3, yaw: f32) -> Option<Quat> {
    if !yaw.is_finite() {
        return None;
    }
    let up = normal.try_normalize()?;
    let reference = if up.dot(Vec3::Z).abs() < 0.95 {
        Vec3::Z
    } else {
        Vec3::X
    };
    let forward = (reference - up * reference.dot(up)).try_normalize()?;
    let yawed_forward = Quat::from_axis_angle(up, yaw) * forward;
    let right = yawed_forward.cross(up).try_normalize()?;
    Some(Quat::from_mat3(&Mat3::from_cols(right, up, -yawed_forward)))
}

/// Atomically transfer a ceramic from inventory to a placed-object record.
pub fn place_inventory_ceramic(
    inventory: &mut Inventory,
    crafted: &[CeramicItem],
    id: CeramicObjectId,
    surface: PlacementSurface,
    rotation: f32,
) -> Result<PlayerPlacedObject, PlacementError> {
    if !inventory.contains_ceramic(id) {
        return Err(PlacementError::NotInInventory);
    }
    let item = crafted
        .iter()
        .find(|item| item.id == id)
        .copied()
        .ok_or(PlacementError::MissingItemData)?;
    if placement_rotation(surface.normal, rotation).is_none() {
        return Err(PlacementError::InvalidSurface);
    }
    // Remove only after all placement preconditions have passed.
    if !inventory.remove_ceramic(id) {
        return Err(PlacementError::NotInInventory);
    }
    Ok(PlayerPlacedObject {
        item,
        location: surface.location,
        rotation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    NotInInventory,
    MissingItemData,
    InvalidSurface,
}

fn spawn_placement_help(mut commands: Commands) {
    commands.spawn((
        PlacementHelpText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn placement_controls(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut mode: ResMut<PlacementMode>,
    mut inventory: ResMut<Inventory>,
    crafted: Res<CraftedCeramics>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut help: Query<&mut Text, With<PlacementHelpText>>,
    mut ghosts: Query<&mut Transform, With<PlacementGhost>>,
) {
    if !mode.active && keyboard.just_pressed(KeyCode::KeyP) {
        mode.selected = inventory.slots().iter().find_map(|slot| match slot {
            InventorySlot::Ceramic(id) => Some(*id),
            _ => None,
        });
        mode.active = mode.selected.is_some();
        mode.rotation = 0.0;
    }
    if !mode.active {
        for mut text in &mut help {
            text.0 = "P: begin placing a ceramic from inventory".to_owned();
        }
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape) || mouse.just_pressed(MouseButton::Right) {
        cancel_placement(&mut commands, &mut mode);
    } else {
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            mode.rotation -= ROTATION_STEP;
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            mode.rotation += ROTATION_STEP;
        }
        if keyboard.just_pressed(KeyCode::BracketLeft)
            || keyboard.just_pressed(KeyCode::BracketRight)
        {
            let ceramics: Vec<_> = inventory
                .slots()
                .iter()
                .filter_map(|slot| match slot {
                    InventorySlot::Ceramic(id) => Some(*id),
                    _ => None,
                })
                .collect();
            if !ceramics.is_empty() {
                let current = ceramics
                    .iter()
                    .position(|id| Some(*id) == mode.selected)
                    .unwrap_or(0);
                let next = if keyboard.just_pressed(KeyCode::BracketRight) {
                    (current + 1) % ceramics.len()
                } else {
                    (current + ceramics.len() - 1) % ceramics.len()
                };
                mode.selected = Some(ceramics[next]);
                if let Some(ghost) = mode.ghost.take() {
                    commands.entity(ghost).despawn();
                }
            }
        }
        mode.surface = cursor_surface(&windows, &cameras);
        if let (Some(surface), Some(id)) = (mode.surface, mode.selected) {
            let item = crafted.items.iter().find(|item| item.id == id).copied();
            if let Some(item) = item {
                let rotation =
                    placement_rotation(surface.normal, mode.rotation).unwrap_or(Quat::IDENTITY);
                let transform = Transform {
                    translation: surface.location.direction.normalize()
                        * (DEFAULT_PLANET_RADIUS + surface.location.altitude),
                    rotation,
                    ..default()
                };
                if let Some(ghost) = mode.ghost {
                    if let Ok(mut ghost_transform) = ghosts.get_mut(ghost) {
                        *ghost_transform = transform;
                    }
                } else {
                    let ghost = spawn_ceramic_ghost(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        item,
                        transform,
                    );
                    commands
                        .entity(ghost)
                        .insert((PlacementGhost, Name::new("Ceramic placement preview")));
                    mode.ghost = Some(ghost);
                }
                if (keyboard.just_pressed(KeyCode::Enter) || mouse.just_pressed(MouseButton::Left))
                    && let Ok(placed) = place_inventory_ceramic(
                        &mut inventory,
                        &crafted.items,
                        id,
                        surface,
                        mode.rotation,
                    )
                {
                    if let Some(ghost) = mode.ghost.take() {
                        commands.entity(ghost).despawn();
                    }
                    let placed_transform = Transform {
                        translation: surface.location.direction.normalize()
                            * (DEFAULT_PLANET_RADIUS + surface.location.altitude),
                        rotation,
                        ..default()
                    };
                    let entity = spawn_ceramic_visual(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        placed.item,
                        placed_transform,
                    );
                    commands.entity(entity).insert(placed);
                    mode.active = false;
                    mode.selected = None;
                }
            }
        } else if let Some(ghost) = mode.ghost.take() {
            commands.entity(ghost).despawn();
        }
    }
    let status = if mode.active {
        "Placement: Enter/click place, Esc/right-click cancel, ←/→ rotate, [/] select ceramic"
    } else {
        "P: place a ceramic from inventory"
    };
    for mut text in &mut help {
        text.0 = status.to_owned();
    }
}

#[derive(Component)]
struct PlacementGhost;

fn cancel_placement(commands: &mut Commands, mode: &mut PlacementMode) {
    if let Some(ghost) = mode.ghost.take() {
        commands.entity(ghost).despawn();
    }
    mode.active = false;
    mode.selected = None;
    mode.surface = None;
}

fn cursor_surface(
    windows: &Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<PlacementSurface> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, transform) = cameras.single().ok()?;
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    raycast_planet_surface(ray.origin, *ray.direction, DEFAULT_PLANET_RADIUS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ceramics::{CeramicForm, CeramicItemTemplate, ClayMaterial, Glaze},
        inventory::CeramicObjectId,
    };

    fn fired(id: u64) -> CeramicItem {
        CeramicItemTemplate {
            form: CeramicForm::Vase,
            clay: ClayMaterial::Red,
            glaze: Glaze::Blue,
            state: crate::ceramics::ProcessingState::Fired,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn ray_hit_produces_outward_normal_and_misses_when_aimed_away() {
        let hit = raycast_planet_surface(Vec3::new(0.0, 0.0, 50.0), Vec3::NEG_Z, 40.0).unwrap();
        assert!((hit.location.direction - Vec3::Z).length() < 1e-6);
        assert!((hit.normal.length() - 1.0).abs() < 1e-6);
        assert!(raycast_planet_surface(Vec3::new(0.0, 0.0, 50.0), Vec3::Z, 40.0).is_none());
    }

    #[test]
    fn placement_orientation_tracks_surface_up_and_rotates_in_tangent_plane() {
        for normal in [Vec3::Y, Vec3::new(0.4, -0.5, 0.7).normalize(), Vec3::NEG_X] {
            let first = placement_rotation(normal, 0.0).unwrap();
            let rotated = placement_rotation(normal, std::f32::consts::FRAC_PI_2).unwrap();
            assert!((first * Vec3::Y).dot(normal) > 0.9999);
            assert!((rotated * Vec3::Y).dot(normal) > 0.9999);
            assert!(((first * -Vec3::Z).dot(rotated * -Vec3::Z)).abs() < 1e-4);
        }
    }

    #[test]
    fn inventory_is_consumed_only_for_successful_placement() {
        let id = CeramicObjectId(7);
        let mut inventory = Inventory::default();
        inventory.add_ceramic(id);
        let surface = raycast_planet_surface(Vec3::Z * 50.0, Vec3::NEG_Z, 40.0).unwrap();
        assert_eq!(
            place_inventory_ceramic(&mut inventory, &[], id, surface, 0.0),
            Err(PlacementError::MissingItemData)
        );
        assert!(inventory.contains_ceramic(id));
        let placed =
            place_inventory_ceramic(&mut inventory, &[fired(id.0)], id, surface, 0.0).unwrap();
        assert_eq!(placed.item.id, id);
        assert!(!inventory.contains_ceramic(id));
        assert_eq!(placed.location, surface.location);
    }
}
