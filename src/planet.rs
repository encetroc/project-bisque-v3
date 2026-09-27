//! Gameplay-facing planet surface coordinates and tile data.
//!
//! Coordinates are expressed in the discrete grid on a cube face, independent
//! of any world-space transform or planet orientation. Out-of-range positions
//! are rejected; they are never silently clamped to a different tile.

use bevy::math::Vec3;

/// Number of tiles along each edge of every cube face in the POC planet.
pub const TILES_PER_FACE: u8 = 24;

/// Starting planet radius recommended by the POC specification.
pub const DEFAULT_PLANET_RADIUS: f32 = 40.0;

/// Lowest permitted authored terrain height, in world units relative to radius.
pub const MIN_TERRAIN_HEIGHT: f32 = -0.5;
/// Highest permitted authored terrain height, in world units relative to radius.
pub const MAX_TERRAIN_HEIGHT: f32 = 1.5;

/// Convert face-local cube coordinates in `[-1, 1]` to a cube-surface point.
///
/// Each face uses a fixed orientation so the same cube corner has the same
/// coordinates regardless of which face is used to name it.
pub fn cube_face_point(face: PlanetFace, u: f32, v: f32) -> Option<Vec3> {
    if !u.is_finite() || !v.is_finite() || !(-1.0..=1.0).contains(&u) || !(-1.0..=1.0).contains(&v)
    {
        return None;
    }

    Some(match face {
        PlanetFace::PositiveX => Vec3::new(1.0, v, -u),
        PlanetFace::NegativeX => Vec3::new(-1.0, v, u),
        PlanetFace::PositiveY => Vec3::new(u, 1.0, -v),
        PlanetFace::NegativeY => Vec3::new(u, -1.0, v),
        PlanetFace::PositiveZ => Vec3::new(u, v, 1.0),
        PlanetFace::NegativeZ => Vec3::new(-u, v, -1.0),
    })
}

/// Project a nonzero cube point onto a sphere of `radius` world units.
///
/// Returns `None` for non-finite/zero points or non-positive/non-finite radii.
pub fn project_cube_to_sphere(cube_point: Vec3, radius: f32) -> Option<Vec3> {
    if !cube_point.is_finite()
        || cube_point.length_squared() == 0.0
        || !radius.is_finite()
        || radius <= 0.0
    {
        return None;
    }

    Some(cube_point.normalize() * radius)
}

/// Project a face-local point onto the planet sphere using the requested radius.
pub fn project_face_to_sphere(face: PlanetFace, u: f32, v: f32, radius: f32) -> Option<Vec3> {
    project_cube_to_sphere(cube_face_point(face, u, v)?, radius)
}

/// One of the six faces of the cube used to define the planet's surface grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl PlanetFace {
    /// All cube faces, in a stable order useful for iteration and serialization.
    pub const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];
}

/// Orientation relative to a face's local grid, not to a global up axis.
///
/// `North` denotes the face-local positive-y direction. Face transitions can
/// rotate this value while preserving the character's surface-relative heading.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceOrientation {
    #[default]
    North,
    East,
    South,
    West,
}

/// A validated tile address on one cube face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoordinate {
    face: PlanetFace,
    x: u8,
    y: u8,
}

impl TileCoordinate {
    /// Create a tile address, rejecting either axis outside `0..TILES_PER_FACE`.
    pub const fn new(face: PlanetFace, x: u8, y: u8) -> Option<Self> {
        if x < TILES_PER_FACE && y < TILES_PER_FACE {
            Some(Self { face, x, y })
        } else {
            None
        }
    }

    pub const fn face(self) -> PlanetFace {
        self.face
    }

    pub const fn x(self) -> u8 {
        self.x
    }

    pub const fn y(self) -> u8 {
        self.y
    }
}

/// A cardinal step in the current face's local tile grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    fn vector(self, face: PlanetFace) -> Vec3 {
        let (u, v) = face_basis(face);
        match self {
            Self::North => v,
            Self::East => u,
            Self::South => -v,
            Self::West => -u,
        }
    }
}

/// A tile address and its surface-relative facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanetCoordinate {
    pub tile: TileCoordinate,
    pub orientation: FaceOrientation,
}

impl PlanetCoordinate {
    pub const fn new(tile: TileCoordinate, orientation: FaceOrientation) -> Self {
        Self { tile, orientation }
    }
}

/// Move one tile in the current face's local cardinal direction, crossing cube
/// edges without exposing face-pair logic to gameplay callers.
pub fn move_coordinate(coordinate: PlanetCoordinate, direction: Direction) -> PlanetCoordinate {
    let tile = coordinate.tile;
    let last = TILES_PER_FACE - 1;
    let is_edge = match direction {
        Direction::North => tile.y == last,
        Direction::East => tile.x == last,
        Direction::South => tile.y == 0,
        Direction::West => tile.x == 0,
    };

    if !is_edge {
        let (x, y) = match direction {
            Direction::North => (tile.x, tile.y + 1),
            Direction::East => (tile.x + 1, tile.y),
            Direction::South => (tile.x, tile.y - 1),
            Direction::West => (tile.x - 1, tile.y),
        };
        return PlanetCoordinate::new(TileCoordinate { x, y, ..tile }, coordinate.orientation);
    }

    let source_normal = face_normal(tile.face);
    let source_u = face_basis(tile.face).0;
    let source_v = face_basis(tile.face).1;
    let target_normal = direction.vector(tile.face);
    let target_face = PlanetFace::ALL
        .into_iter()
        .find(|face| face_normal(*face) == target_normal)
        .expect("each face tangent is another cube face normal");
    let (target_u, target_v) = face_basis(target_face);

    // The shared edge's axis is the source face's other local axis.
    let edge_axis = match direction {
        Direction::East | Direction::West => source_v,
        Direction::North | Direction::South => source_u,
    };
    let turn = if edge_axis.cross(source_normal).dot(target_normal) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let rotate = |vector: Vec3| edge_axis * edge_axis.dot(vector) + turn * edge_axis.cross(vector);

    // Carry the source cell's center across the edge, then express it in the
    // destination face's local basis. The crossing axis is one tile inside.
    let u = (tile.x as f32 + 0.5) / TILES_PER_FACE as f32 * 2.0 - 1.0;
    let v = (tile.y as f32 + 0.5) / TILES_PER_FACE as f32 * 2.0 - 1.0;
    let transverse = match direction {
        Direction::East | Direction::West => source_v * v,
        Direction::North | Direction::South => source_u * u,
    };
    let edge_point = source_normal * (1.0 - 1.0 / TILES_PER_FACE as f32) + transverse;
    let mapped_u = edge_point.dot(target_u);
    let mapped_v = edge_point.dot(target_v);
    let x = grid_index(mapped_u);
    let y = grid_index(mapped_v);
    let next_tile = TileCoordinate {
        face: target_face,
        x,
        y,
    };

    let heading = orientation_direction(coordinate.orientation).vector(tile.face);
    let moved_heading = rotate(heading);
    let next_orientation = direction_for_vector(moved_heading, target_u, target_v);
    PlanetCoordinate::new(next_tile, next_orientation)
}

fn grid_index(value: f32) -> u8 {
    (((value + 1.0) * 0.5 * TILES_PER_FACE as f32).floor() as i32)
        .clamp(0, TILES_PER_FACE as i32 - 1) as u8
}

fn orientation_direction(orientation: FaceOrientation) -> Direction {
    match orientation {
        FaceOrientation::North => Direction::North,
        FaceOrientation::East => Direction::East,
        FaceOrientation::South => Direction::South,
        FaceOrientation::West => Direction::West,
    }
}

fn direction_for_vector(vector: Vec3, u: Vec3, v: Vec3) -> FaceOrientation {
    let (du, dv) = (vector.dot(u), vector.dot(v));
    match (du > 0.5, du < -0.5, dv > 0.5, dv < -0.5) {
        (_, _, true, _) => FaceOrientation::North,
        (true, _, _, _) => FaceOrientation::East,
        (_, _, _, true) => FaceOrientation::South,
        (_, true, _, _) => FaceOrientation::West,
        _ => unreachable!("quarter-turn keeps a cardinal heading cardinal"),
    }
}

fn face_normal(face: PlanetFace) -> Vec3 {
    match face {
        PlanetFace::PositiveX => Vec3::X,
        PlanetFace::NegativeX => Vec3::NEG_X,
        PlanetFace::PositiveY => Vec3::Y,
        PlanetFace::NegativeY => Vec3::NEG_Y,
        PlanetFace::PositiveZ => Vec3::Z,
        PlanetFace::NegativeZ => Vec3::NEG_Z,
    }
}

fn face_basis(face: PlanetFace) -> (Vec3, Vec3) {
    match face {
        PlanetFace::PositiveX => (Vec3::NEG_Z, Vec3::Y),
        PlanetFace::NegativeX => (Vec3::Z, Vec3::Y),
        PlanetFace::PositiveY => (Vec3::X, Vec3::NEG_Z),
        PlanetFace::NegativeY => (Vec3::X, Vec3::Z),
        PlanetFace::PositiveZ => (Vec3::X, Vec3::Y),
        PlanetFace::NegativeZ => (Vec3::NEG_X, Vec3::Y),
    }
}

/// Broad authored region types used by the planet prototype.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    #[default]
    Meadow,
    RedHighlands,
    Coast,
}

/// Resource category that may be present on a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    CommonClay,
    RedClay,
    PaleClay,
    Wood,
    Plant,
    IronMineral,
    Shell,
    Sand,
}

/// Logical surface data for a single tile. Height is in world units relative to
/// the base planet radius; it does not include any world-space transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetTile {
    pub coordinate: TileCoordinate,
    pub biome: Biome,
    /// True only for authored ocean tiles, not walkable coastal land.
    pub deep_water: bool,
    height: f32,
    pub resource_type: Option<ResourceType>,
}

impl PlanetTile {
    pub const fn new(coordinate: TileCoordinate) -> Self {
        Self {
            coordinate,
            biome: Biome::Meadow,
            deep_water: false,
            height: 0.0,
            resource_type: None,
        }
    }

    /// Terrain height relative to the planet's base radius, in world units.
    pub const fn height(&self) -> f32 {
        self.height
    }

    /// Set authored terrain height, clamping finite values to the gentle range.
    /// Non-finite values are rejected and leave the tile unchanged.
    pub fn set_height(&mut self, height: f32) -> bool {
        if !height.is_finite() {
            return false;
        }
        self.height = height.clamp(MIN_TERRAIN_HEIGHT, MAX_TERRAIN_HEIGHT);
        true
    }
}

/// Assign a stable authored region based on the tile's spherical position.
/// The three region centers are the positive Y (studio meadow), positive X
/// (red highlands), and positive Z (coast) directions. Nearest-center masks
/// make each region continuous across cube-face boundaries.
pub fn author_biome(coordinate: TileCoordinate) -> Biome {
    let normal = tile_normal(coordinate);
    let centers = [
        (Biome::Meadow, Vec3::Y),
        (Biome::RedHighlands, Vec3::X),
        (Biome::Coast, Vec3::Z),
    ];
    centers
        .into_iter()
        .max_by(|(_, left), (_, right)| normal.dot(*left).total_cmp(&normal.dot(*right)))
        .expect("the planet has three authored biome regions")
        .0
}

/// Mark the innermost part of the coast as ocean while leaving its outer band
/// as walkable pale shoreline. This mask is stable across face transitions.
pub fn is_authored_deep_water(coordinate: TileCoordinate) -> bool {
    author_biome(coordinate) == Biome::Coast && tile_normal(coordinate).dot(Vec3::Z) >= 0.86
}

fn tile_normal(coordinate: TileCoordinate) -> Vec3 {
    let u = (coordinate.x() as f32 + 0.5) / TILES_PER_FACE as f32 * 2.0 - 1.0;
    let v = (coordinate.y() as f32 + 0.5) / TILES_PER_FACE as f32 * 2.0 - 1.0;
    cube_face_point(coordinate.face(), u, v)
        .expect("validated tile coordinates map onto the cube face")
        .normalize()
}

/// Build a tile populated with this planet's authored biome and water mask.
pub fn authored_planet_tile(coordinate: TileCoordinate) -> PlanetTile {
    let mut tile = PlanetTile::new(coordinate);
    tile.biome = author_biome(coordinate);
    tile.deep_water = is_authored_deep_water(coordinate);
    tile
}

/// World-space sample of a tile-authored point on the planet surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSample {
    /// Position relative to the planet center, including terrain elevation.
    pub position: Vec3,
    /// Authored terrain elevation above the base planet radius.
    pub height: f32,
    /// Normalized outward radial direction at this sample.
    pub normal: Vec3,
}

/// Sample the center of a tile using its authored height and the planet radius.
///
/// Returns `None` when the radius is invalid or the radius plus height is not
/// positive. The tile is a flat patch for this POC, so its normal is radial.
pub fn sample_tile_surface(tile: &PlanetTile, radius: f32) -> Option<SurfaceSample> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }

    let coordinate = tile.coordinate;
    let normal = tile_normal(coordinate);
    let height = tile.height();
    let surface_radius = radius + height;
    if surface_radius <= 0.0 {
        return None;
    }

    Some(SurfaceSample {
        position: normal * surface_radius,
        height,
        normal,
    })
}

impl Default for PlanetTile {
    fn default() -> Self {
        Self::new(TileCoordinate {
            face: PlanetFace::PositiveX,
            x: 0,
            y: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_planet_has_three_stable_regions_and_distinguishes_ocean_from_shore() {
        let mut biome_counts = [0usize; 3];
        let mut deep_water_count = 0;
        let mut walkable_coast_count = 0;
        for face in PlanetFace::ALL {
            for y in 0..TILES_PER_FACE {
                for x in 0..TILES_PER_FACE {
                    let coordinate = TileCoordinate::new(face, x, y).unwrap();
                    let tile = authored_planet_tile(coordinate);
                    assert_eq!(tile.biome, author_biome(coordinate));
                    assert_eq!(tile.deep_water, is_authored_deep_water(coordinate));
                    match tile.biome {
                        Biome::Meadow => biome_counts[0] += 1,
                        Biome::RedHighlands => biome_counts[1] += 1,
                        Biome::Coast => {
                            biome_counts[2] += 1;
                            if tile.deep_water {
                                deep_water_count += 1;
                            } else {
                                walkable_coast_count += 1;
                            }
                        }
                    }
                    assert!(!tile.deep_water || tile.biome == Biome::Coast);
                }
            }
        }

        assert!(biome_counts.into_iter().all(|count| count > 0));
        assert!(deep_water_count > 0);
        assert!(walkable_coast_count > 0);
        assert_eq!(
            author_biome(TileCoordinate::new(PlanetFace::PositiveY, 11, 14).unwrap()),
            Biome::Meadow
        );
        assert_eq!(
            author_biome(TileCoordinate::new(PlanetFace::PositiveX, 11, 11).unwrap()),
            Biome::RedHighlands
        );
        assert_eq!(
            author_biome(TileCoordinate::new(PlanetFace::PositiveZ, 11, 11).unwrap()),
            Biome::Coast
        );
    }

    #[test]
    fn cube_faces_project_to_requested_radius_with_finite_symmetric_results() {
        for face in PlanetFace::ALL {
            let point = project_face_to_sphere(face, 0.25, -0.5, DEFAULT_PLANET_RADIUS).unwrap();
            assert!(point.is_finite());
            assert!((point.length() - DEFAULT_PLANET_RADIUS).abs() < 1e-5);
        }

        let positive = project_face_to_sphere(PlanetFace::PositiveZ, 0.4, -0.2, 17.0).unwrap();
        let negative = project_face_to_sphere(PlanetFace::NegativeZ, 0.4, 0.2, 17.0).unwrap();
        assert!((positive + negative).length() < 1e-5);

        let center = project_cube_to_sphere(Vec3::X, 0.0);
        assert!(center.is_none());
        assert!(project_cube_to_sphere(Vec3::ZERO, DEFAULT_PLANET_RADIUS).is_none());
        assert!(project_face_to_sphere(PlanetFace::PositiveX, f32::NAN, 0.0, 40.0).is_none());
    }

    #[test]
    fn all_face_representations_of_cube_corners_project_identically() {
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let mut projections = Vec::new();
                    if x > 0.0 {
                        projections.push(
                            project_face_to_sphere(PlanetFace::PositiveX, -z, y, 40.0).unwrap(),
                        );
                    } else {
                        projections.push(
                            project_face_to_sphere(PlanetFace::NegativeX, z, y, 40.0).unwrap(),
                        );
                    }
                    if y > 0.0 {
                        projections.push(
                            project_face_to_sphere(PlanetFace::PositiveY, x, -z, 40.0).unwrap(),
                        );
                    } else {
                        projections.push(
                            project_face_to_sphere(PlanetFace::NegativeY, x, z, 40.0).unwrap(),
                        );
                    }
                    if z > 0.0 {
                        projections.push(
                            project_face_to_sphere(PlanetFace::PositiveZ, x, y, 40.0).unwrap(),
                        );
                    } else {
                        projections.push(
                            project_face_to_sphere(PlanetFace::NegativeZ, -x, y, 40.0).unwrap(),
                        );
                    }

                    for projection in &projections[1..] {
                        assert!(projections[0].distance(*projection) < 1e-5);
                    }
                }
            }
        }
    }

    #[test]
    fn all_six_faces_are_enumerated() {
        assert_eq!(PlanetFace::ALL.len(), 6);
        for (index, face) in PlanetFace::ALL.iter().enumerate() {
            assert!(!PlanetFace::ALL[..index].contains(face));
        }
    }

    #[test]
    fn tile_coordinates_accept_inclusive_grid_edges_and_reject_out_of_bounds() {
        assert!(TileCoordinate::new(PlanetFace::NegativeZ, 0, 0).is_some());
        assert_eq!(
            TileCoordinate::new(
                PlanetFace::PositiveY,
                TILES_PER_FACE - 1,
                TILES_PER_FACE - 1
            ),
            Some(TileCoordinate {
                face: PlanetFace::PositiveY,
                x: 23,
                y: 23,
            })
        );
        assert!(TileCoordinate::new(PlanetFace::PositiveX, TILES_PER_FACE, 0).is_none());
        assert!(TileCoordinate::new(PlanetFace::PositiveX, 0, TILES_PER_FACE).is_none());
    }

    #[test]
    fn tile_defaults_are_defined_and_fields_are_mutable() {
        let coordinate = TileCoordinate::new(PlanetFace::NegativeY, 7, 12).unwrap();
        let mut tile = PlanetTile::new(coordinate);
        assert_eq!(tile.coordinate, coordinate);
        assert_eq!(tile.biome, Biome::Meadow);
        assert!(!tile.deep_water);
        assert_eq!(tile.height(), 0.0);
        assert_eq!(tile.resource_type, None);

        tile.biome = Biome::RedHighlands;
        assert!(tile.set_height(1.25));
        tile.resource_type = Some(ResourceType::IronMineral);
        assert_eq!(tile.biome, Biome::RedHighlands);
        assert_eq!(tile.height(), 1.25);
        assert_eq!(tile.resource_type, Some(ResourceType::IronMineral));
    }

    #[test]
    fn terrain_heights_are_bounded_and_reject_non_finite_values() {
        let coordinate = TileCoordinate::new(PlanetFace::PositiveX, 0, 0).unwrap();
        let mut tile = PlanetTile::new(coordinate);
        assert!(tile.set_height(-100.0));
        assert_eq!(tile.height(), MIN_TERRAIN_HEIGHT);
        assert!(tile.set_height(100.0));
        assert_eq!(tile.height(), MAX_TERRAIN_HEIGHT);
        assert!(!tile.set_height(f32::NAN));
        assert_eq!(tile.height(), MAX_TERRAIN_HEIGHT);
    }

    #[test]
    fn flat_and_varied_tiles_sample_radius_height_and_outward_normal() {
        let flat_coordinate = TileCoordinate::new(PlanetFace::PositiveZ, 11, 7).unwrap();
        let flat = PlanetTile::new(flat_coordinate);
        let flat_sample = sample_tile_surface(&flat, 40.0).unwrap();
        assert_eq!(flat_sample.height, 0.0);
        assert!((flat_sample.position.length() - 40.0).abs() < 1e-5);

        let varied_coordinate = TileCoordinate::new(PlanetFace::NegativeY, 3, 19).unwrap();
        let mut varied = PlanetTile::new(varied_coordinate);
        assert!(varied.set_height(1.25));
        let varied_sample = sample_tile_surface(&varied, 30.0).unwrap();
        assert_eq!(varied_sample.height, 1.25);
        assert!((varied_sample.position.length() - 31.25).abs() < 1e-5);
        assert!((varied_sample.normal.length() - 1.0).abs() < 1e-6);
        assert!(varied_sample.normal.dot(varied_sample.position) > 0.0);
        assert!(varied_sample.normal.dot(Vec3::NEG_Y) > 0.0);
    }

    #[test]
    fn surface_sampling_rejects_invalid_or_non_positive_surface_radius() {
        let coordinate = TileCoordinate::new(PlanetFace::PositiveX, 0, 0).unwrap();
        let mut tile = PlanetTile::new(coordinate);
        assert!(sample_tile_surface(&tile, 0.0).is_none());
        assert!(sample_tile_surface(&tile, f32::INFINITY).is_none());
        assert!(tile.set_height(-0.5));
        assert!(sample_tile_surface(&tile, 0.25).is_none());
    }

    #[test]
    fn every_face_edge_transition_is_valid_and_reversible() {
        for face in PlanetFace::ALL {
            for direction in Direction::ALL {
                for offset in 0..TILES_PER_FACE {
                    let (x, y) = match direction {
                        Direction::North => (offset, TILES_PER_FACE - 1),
                        Direction::East => (TILES_PER_FACE - 1, offset),
                        Direction::South => (offset, 0),
                        Direction::West => (0, offset),
                    };
                    let start = PlanetCoordinate::new(
                        TileCoordinate::new(face, x, y).unwrap(),
                        FaceOrientation::East,
                    );
                    let moved = move_coordinate(start, direction);
                    assert_ne!(moved.tile.face(), face);
                    assert!(moved.tile.x() < TILES_PER_FACE);
                    assert!(moved.tile.y() < TILES_PER_FACE);

                    let reverse = Direction::ALL
                        .into_iter()
                        .map(|candidate| move_coordinate(moved, candidate))
                        .find(|candidate| candidate.tile == start.tile)
                        .expect("the adjoining face must provide a reverse edge step");
                    assert_eq!(reverse, start, "{face:?} {direction:?} at {offset}");
                }
            }
        }
    }

    #[test]
    fn movement_inside_face_preserves_orientation_and_changes_one_axis() {
        let start = PlanetCoordinate::new(
            TileCoordinate::new(PlanetFace::PositiveZ, 10, 10).unwrap(),
            FaceOrientation::West,
        );
        assert_eq!(move_coordinate(start, Direction::East).tile.x(), 11);
        assert_eq!(move_coordinate(start, Direction::North).tile.y(), 11);
        assert_eq!(
            move_coordinate(start, Direction::East).orientation,
            start.orientation
        );
    }

    #[test]
    fn orientation_is_surface_relative_and_has_four_headings() {
        let tile = TileCoordinate::new(PlanetFace::PositiveY, 4, 9).unwrap();
        let coordinate = PlanetCoordinate::new(tile, FaceOrientation::West);
        assert_eq!(coordinate.tile.face(), PlanetFace::PositiveY);
        assert_eq!(coordinate.orientation, FaceOrientation::West);
        assert_eq!(FaceOrientation::default(), FaceOrientation::North);
        assert_ne!(FaceOrientation::North, FaceOrientation::East);
        assert_ne!(FaceOrientation::East, FaceOrientation::South);
        assert_ne!(FaceOrientation::South, FaceOrientation::West);
    }
}
