//! Surface-relative player input and bounded movement.

use bevy::prelude::*;

use crate::planet::{DEFAULT_PLANET_RADIUS, SurfaceSample};
use crate::surface_transform::{SurfaceLocation, surface_transform};

/// Walking speed in world units per second.
pub const WALK_SPEED: f32 = 4.0;
/// Running speed in world units per second.
pub const RUN_SPEED: f32 = 7.0;

/// Marks an actor as controlled by the surface movement system.
#[derive(Component, Debug, Clone, Copy)]
pub struct SurfacePlayer {
    pub location: SurfaceLocation,
    /// Desired tangent-plane forward direction.
    pub heading: Vec3,
    /// Radius of the planet this actor moves on.
    pub planet_radius: f32,
    /// Terrain elevation beneath the actor, relative to the planet radius.
    pub terrain_height: f32,
}

impl SurfacePlayer {
    pub fn new(location: SurfaceLocation, heading: Vec3, planet_radius: f32) -> Self {
        Self {
            location,
            heading,
            planet_radius,
            terrain_height: 0.0,
        }
    }
}

/// Installs keyboard-driven movement relative to the active surface camera.
pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_surface_players);
    }
}

pub(crate) fn move_surface_players(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<(&mut SurfacePlayer, &mut Transform)>,
) {
    let mut input = Vec2::ZERO;
    input.x += f32::from(keyboard.pressed(KeyCode::KeyD));
    input.x -= f32::from(keyboard.pressed(KeyCode::KeyA));
    input.y += f32::from(keyboard.pressed(KeyCode::KeyW));
    input.y -= f32::from(keyboard.pressed(KeyCode::KeyS));
    if input == Vec2::ZERO {
        return;
    }
    let running = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let speed = if running { RUN_SPEED } else { WALK_SPEED };
    let distance = speed * time.delta_secs();
    let camera_basis = cameras.single().ok().map(|camera| {
        let rotation = camera.compute_transform().rotation;
        (rotation * -Vec3::Z, rotation * Vec3::X)
    });

    for (mut player, mut transform) in &mut players {
        let normal = player.location.direction.try_normalize().unwrap_or(Vec3::Y);
        let fallback_forward = (player.heading - normal * player.heading.dot(normal))
            .try_normalize()
            .unwrap_or_else(|| tangent_from_normal(normal));
        let (camera_forward, camera_right) =
            camera_basis.unwrap_or((fallback_forward, fallback_forward.cross(normal)));
        let Some(tangent) = camera_relative_tangent(
            normal,
            camera_forward,
            camera_right,
            fallback_forward,
            input,
        ) else {
            continue;
        };
        let Some((direction, heading)) = advance_on_sphere(
            normal,
            fallback_forward,
            tangent,
            distance.min(speed * 0.1),
            player.planet_radius + player.terrain_height,
        ) else {
            continue;
        };
        player.location.direction = direction;
        player.heading = heading;
        let sample = SurfaceSample {
            position: direction * (player.planet_radius + player.terrain_height),
            height: player.terrain_height,
            normal: direction,
        };
        if let Some(updated) = surface_transform(
            player.location,
            Vec3::ZERO,
            player.planet_radius,
            &sample,
            heading,
        ) {
            *transform = updated;
        }
    }
}

/// Map screen-relative input onto the local tangent plane. If the camera points
/// directly along the surface normal, its projected right vector supplies a
/// stable forward direction; invalid or degenerate camera vectors use the
/// player's transported heading instead.
fn camera_relative_tangent(
    normal: Vec3,
    camera_forward: Vec3,
    camera_right: Vec3,
    fallback_forward: Vec3,
    input: Vec2,
) -> Option<Vec3> {
    if !normal.is_finite()
        || !camera_forward.is_finite()
        || !camera_right.is_finite()
        || !fallback_forward.is_finite()
        || !input.is_finite()
    {
        return None;
    }
    let normal = normal.try_normalize()?;
    let fallback_forward = (fallback_forward - normal * fallback_forward.dot(normal))
        .try_normalize()
        .unwrap_or_else(|| tangent_from_normal(normal));
    let projected_right = (camera_right - normal * camera_right.dot(normal)).try_normalize();
    let forward = (camera_forward - normal * camera_forward.dot(normal))
        .try_normalize()
        .or_else(|| projected_right.map(|right| normal.cross(right)))
        .unwrap_or(fallback_forward);
    let right = forward.cross(normal).try_normalize()?;
    let tangent = right * input.x + forward * input.y;
    tangent.is_finite().then_some(tangent)
}

/// Advance a radial location along a great circle and parallel-transport its
/// heading. The distance is clamped to 10% of a radius to keep each frame
/// bounded even after a long stall.
pub fn advance_on_sphere(
    direction: Vec3,
    heading: Vec3,
    tangent_input: Vec3,
    distance: f32,
    surface_radius: f32,
) -> Option<(Vec3, Vec3)> {
    if !direction.is_finite()
        || !heading.is_finite()
        || !tangent_input.is_finite()
        || !distance.is_finite()
        || distance < 0.0
        || !surface_radius.is_finite()
        || surface_radius <= 0.0
    {
        return None;
    }
    let up = direction.try_normalize()?;
    let forward = (heading - up * heading.dot(up)).try_normalize()?;
    let tangent = (tangent_input - up * tangent_input.dot(up)).try_normalize()?;
    if distance == 0.0 {
        return Some((up, forward));
    }
    let angle = (distance.min(surface_radius * 0.25) / surface_radius).min(0.25);
    let next_up = (up * angle.cos() + tangent * angle.sin()).try_normalize()?;
    let rotation_axis = up.cross(tangent).try_normalize()?;
    let transported = Quat::from_axis_angle(rotation_axis, angle) * forward;
    let next_forward = (transported - next_up * transported.dot(next_up)).try_normalize()?;
    Some((next_up, next_forward))
}

fn tangent_from_normal(normal: Vec3) -> Vec3 {
    let reference = if normal.dot(Vec3::Y).abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    (reference - normal * reference.dot(normal)).normalize()
}

/// Default movement-plugin planet radius, for callers building the POC scene.
pub const PLAYER_PLANET_RADIUS: f32 = DEFAULT_PLANET_RADIUS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_relative_mapping_tracks_yaw_across_surface_normals() {
        for normal in [
            Vec3::Y,
            Vec3::new(0.2, 0.96, 0.1).normalize(),
            Vec3::new(0.7, -0.3, 0.64).normalize(),
            Vec3::new(0.001, 1.0, 0.0).normalize(),
            Vec3::X,
        ] {
            let heading = tangent_from_normal(normal);
            let surface_rotation = Quat::from_rotation_arc(Vec3::Y, normal);
            for yaw in [0.0, 0.7, 1.8, 4.2] {
                let camera_rotation = surface_rotation * Quat::from_rotation_y(yaw);
                let camera_forward = camera_rotation * -Vec3::Z;
                let camera_right = camera_rotation * Vec3::X;
                let forward =
                    camera_relative_tangent(normal, camera_forward, camera_right, heading, Vec2::Y)
                        .unwrap()
                        .normalize();
                let right =
                    camera_relative_tangent(normal, camera_forward, camera_right, heading, Vec2::X)
                        .unwrap()
                        .normalize();
                let expected_forward =
                    (camera_forward - normal * camera_forward.dot(normal)).normalize();
                let expected_right = (camera_right - normal * camera_right.dot(normal)).normalize();
                assert!(forward.dot(expected_forward) > 0.9999);
                assert!(right.dot(expected_right) > 0.9999);
                assert!(forward.dot(normal).abs() < 1e-5);
                assert!(right.dot(normal).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn degenerate_camera_projection_falls_back_without_nan() {
        let normal = Vec3::Y;
        let fallback = Vec3::X;
        let from_vertical_camera =
            camera_relative_tangent(normal, normal, Vec3::X, fallback, Vec2::Y).unwrap();
        assert!(from_vertical_camera.is_finite());
        assert!(from_vertical_camera.normalize().dot(Vec3::NEG_Z) > 0.9999);

        let from_degenerate_camera =
            camera_relative_tangent(normal, normal, normal, fallback, Vec2::Y).unwrap();
        assert!(from_degenerate_camera.is_finite());
        assert!(from_degenerate_camera.normalize().dot(fallback) > 0.9999);
    }

    #[test]
    fn movement_crosses_every_cube_face_seam_without_flipping() {
        let faces = [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ];
        for from in faces {
            for to in faces {
                if from.dot(to) != 0.0 {
                    continue;
                }
                let seam = (from + to).normalize();
                let tangent = (to - seam * to.dot(seam)).normalize();
                let (next, heading) = advance_on_sphere(seam, tangent, tangent, 0.2, 40.0).unwrap();
                assert!(next.is_finite());
                assert!((next.length() - 1.0).abs() < 1e-6);
                assert!((heading.length() - 1.0).abs() < 1e-6);
                assert!(heading.dot(next).abs() < 1e-6);
                assert!(next.dot(to) > seam.dot(to), "must cross {from:?} to {to:?}");
            }
        }
    }

    #[test]
    fn crossing_a_pole_preserves_a_continuous_tangent_frame() {
        let near_pole = Vec3::new(0.001, 1.0, 0.0).normalize();
        let heading = Vec3::NEG_Y - near_pole * Vec3::NEG_Y.dot(near_pole);
        let (after, transported) =
            advance_on_sphere(near_pole, heading, heading, 0.2, 1.0).unwrap();
        assert!(after.is_finite() && transported.is_finite());
        assert!(after.dot(near_pole) < 1.0);
        assert!(transported.dot(after).abs() < 1e-5);
        assert!(transported.dot(heading.normalize()) > 0.9);
    }

    #[test]
    fn walk_and_run_are_distinct_and_each_frame_step_is_bounded() {
        assert!(RUN_SPEED > WALK_SPEED);
        assert!(WALK_SPEED > 0.0 && RUN_SPEED <= 10.0);
        let (walk, _) = advance_on_sphere(Vec3::Z, Vec3::X, Vec3::X, WALK_SPEED, 40.0).unwrap();
        let (run, _) = advance_on_sphere(Vec3::Z, Vec3::X, Vec3::X, RUN_SPEED, 40.0).unwrap();
        assert!(run.distance(Vec3::Z) > walk.distance(Vec3::Z));
        let (bounded, _) = advance_on_sphere(Vec3::Z, Vec3::X, Vec3::X, 1000.0, 40.0).unwrap();
        assert!(bounded.dot(Vec3::Z) > 0.95);
    }

    #[test]
    fn invalid_or_degenerate_movement_inputs_are_rejected() {
        assert!(advance_on_sphere(Vec3::ZERO, Vec3::X, Vec3::Y, 1.0, 40.0).is_none());
        assert!(advance_on_sphere(Vec3::Z, Vec3::Z, Vec3::X, 1.0, 40.0).is_none());
        assert!(advance_on_sphere(Vec3::Z, Vec3::X, Vec3::X, f32::NAN, 40.0).is_none());
    }
}
