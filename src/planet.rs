//! Gameplay-facing planet surface coordinates and tile data.
//!
//! Coordinates are expressed in the discrete grid on a cube face, independent
//! of any world-space transform or planet orientation. Out-of-range positions
//! are rejected; they are never silently clamped to a different tile.

/// Number of tiles along each edge of every cube face in the POC planet.
pub const TILES_PER_FACE: u8 = 24;

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
    pub height: f32,
    pub resource_type: Option<ResourceType>,
}

impl PlanetTile {
    pub const fn new(coordinate: TileCoordinate) -> Self {
        Self {
            coordinate,
            biome: Biome::Meadow,
            height: 0.0,
            resource_type: None,
        }
    }
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
        assert_eq!(tile.height, 0.0);
        assert_eq!(tile.resource_type, None);

        tile.biome = Biome::RedHighlands;
        tile.height = 1.25;
        tile.resource_type = Some(ResourceType::IronMineral);
        assert_eq!(tile.biome, Biome::RedHighlands);
        assert_eq!(tile.height, 1.25);
        assert_eq!(tile.resource_type, Some(ResourceType::IronMineral));
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
