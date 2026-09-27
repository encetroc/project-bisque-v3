//! A soft, surface-oriented camera rig for the spherical planet.

use bevy::prelude::*;

use crate::player_movement::SurfacePlayer;

/// Distance from the player to the camera, in world units.
pub const CAMERA_DISTANCE: f32 = 12.0;
/// Camera elevation above the local tangent plane.
pub const CAMERA_PITCH: f32 = 50.0_f32.to_radians();
/// Perspective field of view, in radians.
pub const CAMERA_FOV: f32 = 30.0_f32.to_radians();
const FOLLOW_SPEED: f32 = 8.0;

#[derive(Component)]
struct CameraTarget {
    pivot: Entity,
    initialized: bool,
}

#[derive(Component)]
struct CameraPivot;

/// Installs a pivot-based camera whose up axis follows the player's surface frame.
pub struct SurfaceCameraFollowPlugin;

impl Plugin for SurfaceCameraFollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_rig)
            .add_systems(Update, follow_surface_player);
    }
}

fn spawn_camera_rig(mut commands: Commands) {
    let pivot = commands.spawn((CameraPivot, Transform::IDENTITY)).id();
    let target = commands
        .spawn((
            Name::new("Surface camera target"),
            CameraTarget {
                pivot,
                initialized: false,
            },
            Transform::IDENTITY,
        ))
        .id();
    commands.entity(pivot).insert(ChildOf(target));

    let camera_position = Vec3::new(
        0.0,
        CAMERA_DISTANCE * CAMERA_PITCH.sin(),
        CAMERA_DISTANCE * CAMERA_PITCH.cos(),
    );
    commands.spawn((
        Name::new("Surface follow camera"),
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: CAMERA_FOV,
            ..default()
        }),
        Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
        ChildOf(pivot),
    ));
}

fn follow_surface_player(
    time: Res<Time>,
    players: Query<(&SurfacePlayer, &Transform)>,
    mut targets: Query<(&mut Transform, &mut CameraTarget)>,
    mut pivots: Query<&mut Transform, With<CameraPivot>>,
) {
    let Ok((_player, player_transform)) = players.single() else {
        return;
    };
    let Ok((mut target_transform, mut target)) = targets.single_mut() else {
        return;
    };
    let Ok(mut pivot_transform) = pivots.get_mut(target.pivot) else {
        return;
    };

    if !target.initialized {
        target_transform.translation = player_transform.translation;
        target.initialized = true;
    } else {
        target_transform.translation = smooth_follow(
            target_transform.translation,
            player_transform.translation,
            time.delta_secs(),
        );
    }

    // The surface movement transform is built from the sampled normal and tangent
    // heading, so copying its rotation carries the camera frame over the sphere.
    pivot_transform.rotation = player_transform.rotation;
}

fn smooth_follow(current: Vec3, desired: Vec3, delta_secs: f32) -> Vec3 {
    let blend = 1.0 - (-FOLLOW_SPEED * delta_secs).exp();
    current.lerp(desired, blend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_transform::{SurfaceLocation, surface_transform};

    #[test]
    fn camera_pivot_up_tracks_player_surface_orientation_at_multiple_latitudes() {
        for normal in [
            Vec3::new(0.2, 0.96, 0.1).normalize(),
            Vec3::new(0.7, -0.3, 0.64).normalize(),
            Vec3::new(-0.1, -0.8, 0.58).normalize(),
        ] {
            let heading = (Vec3::X - normal * Vec3::X.dot(normal)).normalize();
            let surface = crate::planet::SurfaceSample {
                position: normal * 40.0,
                height: 0.0,
                normal,
            };
            let player_transform = surface_transform(
                SurfaceLocation::new(normal, 0.0),
                Vec3::ZERO,
                40.0,
                &surface,
                heading,
            )
            .unwrap();
            let camera_up = player_transform.rotation * Vec3::Y;
            assert!(camera_up.dot(normal) > 0.9999);
            assert!(camera_up.dot(Vec3::Y).abs() < 0.99);
        }
    }

    #[test]
    fn follow_is_soft_and_converges_without_hard_attachment() {
        let current = Vec3::ZERO;
        let desired = Vec3::new(10.0, 0.0, 0.0);
        let first_step = smooth_follow(current, desired, 1.0 / 60.0);
        assert!(first_step.x > 0.0 && first_step.x < desired.x);
        let settled = smooth_follow(first_step, desired, 10.0);
        assert!(settled.distance(desired) < 1e-4);
    }

    #[test]
    fn follow_camera_is_elevated_perspective_with_spec_defaults() {
        assert_eq!(CAMERA_DISTANCE, 12.0);
        assert!((CAMERA_PITCH.to_degrees() - 50.0).abs() < 1e-5);
        assert!((CAMERA_FOV.to_degrees() - 30.0).abs() < 1e-5);
        let offset = Vec3::new(
            0.0,
            CAMERA_DISTANCE * CAMERA_PITCH.sin(),
            CAMERA_DISTANCE * CAMERA_PITCH.cos(),
        );
        assert!((offset.length() - CAMERA_DISTANCE).abs() < 1e-5);
        assert!(offset.y > 0.0 && offset.z > 0.0);
    }
}
