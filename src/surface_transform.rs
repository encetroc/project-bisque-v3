//! Convert planet-relative locations and tangent headings into world transforms.
//!
//! Altitude is measured above the planet's base radius. Terrain elevation is
//! supplied by the surface sample and actor/prop clearance can be added to the
//! location altitude. The sampled normal, rather than a world-axis convention,
//! defines the transform's local up direction.

use bevy::prelude::{Quat, Transform, Vec3};

use crate::planet::SurfaceSample;

/// A location expressed relative to a spherical planet's center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceLocation {
    /// Radial direction from the planet center; need not already be normalized.
    pub direction: Vec3,
    /// Height above the planet's base radius, in world units.
    pub altitude: f32,
}

impl SurfaceLocation {
    pub const fn new(direction: Vec3, altitude: f32) -> Self {
        Self {
            direction,
            altitude,
        }
    }
}

/// Build a transform at a surface-relative location, oriented to the sampled
/// surface normal and a desired tangent-plane forward direction.
///
/// `surface` should be sampled at `location.direction`. `planet_center` allows
/// the planet to be positioned away from the world origin. The surface's
/// authored height is included in the radial position, while `location`'s
/// altitude can account for an actor's/prop's clearance above the surface.
/// Bevy's local forward axis is `-Z` and local up axis is `+Y`.
///
/// Returns `None` for invalid or degenerate inputs, including a heading parallel
/// to the surface normal.
pub fn surface_transform(
    location: SurfaceLocation,
    planet_center: Vec3,
    planet_radius: f32,
    surface: &SurfaceSample,
    tangent_forward: Vec3,
) -> Option<Transform> {
    if !location.direction.is_finite()
        || location.direction.length_squared() <= f32::EPSILON
        || !location.altitude.is_finite()
        || !planet_center.is_finite()
        || !planet_radius.is_finite()
        || planet_radius <= 0.0
        || !surface.height.is_finite()
        || !surface.normal.is_finite()
        || !tangent_forward.is_finite()
    {
        return None;
    }

    let up = surface.normal.try_normalize()?;
    let projected_forward = tangent_forward - up * tangent_forward.dot(up);
    if projected_forward.length_squared() <= 1e-6 {
        return None;
    }
    let forward = projected_forward.try_normalize()?;
    let radius = planet_radius + surface.height + location.altitude;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }

    // The basis columns are local +X (right), +Y (up), and +Z (backward).
    let right = forward.cross(up).try_normalize()?;
    let backward = -forward;
    let rotation = Quat::from_mat3(&bevy::math::Mat3::from_cols(right, up, backward));
    let position = planet_center + location.direction.normalize() * radius;

    Some(Transform {
        translation: position,
        rotation,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::{PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface};

    fn sample(face: PlanetFace, x: u8, y: u8, radius: f32) -> SurfaceSample {
        let tile = PlanetTile::new(TileCoordinate::new(face, x, y).unwrap());
        sample_tile_surface(&tile, radius).unwrap()
    }

    #[test]
    fn positions_and_local_up_follow_sampled_normals_at_non_equatorial_locations() {
        let radius = 40.0;
        let cases = [
            (PlanetFace::PositiveX, 5, 8),
            (PlanetFace::NegativeY, 17, 6),
            (PlanetFace::NegativeZ, 7, 18),
        ];

        for (face, x, y) in cases {
            let surface = sample(face, x, y, radius);
            let location = SurfaceLocation::new(surface.normal, 2.25);
            let transform =
                surface_transform(location, Vec3::ZERO, radius, &surface, Vec3::Z).unwrap();

            assert!((transform.translation.length() - (radius + 2.25)).abs() < 1e-4);
            assert!((transform.rotation * Vec3::Y).dot(surface.normal) > 0.99999);
            assert!((transform.rotation * -Vec3::Z).dot(surface.normal).abs() < 1e-5);
        }
    }

    #[test]
    fn supports_translated_planets_and_terrain_height() {
        let radius = 30.0;
        let center = Vec3::new(12.0, -5.0, 8.0);
        let coordinate = TileCoordinate::new(PlanetFace::PositiveY, 4, 16).unwrap();
        let mut tile = PlanetTile::new(coordinate);
        assert!(tile.set_height(1.25));
        let surface = sample_tile_surface(&tile, radius).unwrap();
        let location = SurfaceLocation::new(surface.normal, 0.75);
        let transform = surface_transform(location, center, radius, &surface, Vec3::X).unwrap();

        assert!((transform.translation.distance(center) - 32.0).abs() < 1e-4);
        assert!((transform.rotation * Vec3::Y).dot(surface.normal) > 0.99999);
    }

    #[test]
    fn tangent_heading_sets_forward_without_global_up_assumptions() {
        let surface = sample(PlanetFace::PositiveZ, 13, 9, 40.0);
        let heading = Vec3::X;
        let transform = surface_transform(
            SurfaceLocation::new(surface.normal, 0.0),
            Vec3::ZERO,
            40.0,
            &surface,
            heading,
        )
        .unwrap();
        let actual_forward = transform.rotation * -Vec3::Z;

        assert!(actual_forward.dot(surface.normal).abs() < 1e-5);
        assert!(actual_forward.dot(heading).abs() > 0.99);
    }

    #[test]
    fn invalid_location_or_degenerate_heading_is_rejected() {
        let surface = sample(PlanetFace::PositiveY, 12, 12, 40.0);
        assert!(
            surface_transform(
                SurfaceLocation::new(Vec3::ZERO, 0.0),
                Vec3::ZERO,
                40.0,
                &surface,
                Vec3::X,
            )
            .is_none()
        );
        assert!(
            surface_transform(
                SurfaceLocation::new(surface.normal, 0.0),
                Vec3::ZERO,
                40.0,
                &surface,
                surface.normal,
            )
            .is_none()
        );
    }
}
