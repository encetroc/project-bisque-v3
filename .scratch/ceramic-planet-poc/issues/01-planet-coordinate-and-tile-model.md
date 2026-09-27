# 01 — Define planet coordinates and logical tiles

Status: ready-for-agent
Blocked by: none

## Scope
Define cube-face identity, face-local coordinates, orientation, and `PlanetTile` data (biome, height, resource type). Keep gameplay-facing surface coordinates independent of world-space transforms. No rendering or transition logic.

## Acceptance criteria
- Coordinate and tile types represent all six faces and valid tile positions.
- Invalid coordinates are rejected or safely clamped by documented rules.
- Orientation can be represented without assuming global Y is up.

## Tests
- Unit tests cover face enumeration, coordinate bounds, and tile defaults/fields.
- Compile/test the planet module.
