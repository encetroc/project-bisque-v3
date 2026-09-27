# 02 — Project cube coordinates onto sphere

Status: ready-for-agent
Blocked by: 01

## Scope
Implement deterministic cube-face vertex to unit-sphere projection and radius scaling. Do not build meshes or terrain.

## Acceptance criteria
- Projection accepts points from every cube face and a radius.
- Output has requested radius within numeric tolerance; center/cube vertices map consistently.
- Radius is configurable (default near 40), not hard-coded into gameplay.

## Tests
- Unit tests check radius, symmetry, finite values, and face-edge agreement at shared cube corners.