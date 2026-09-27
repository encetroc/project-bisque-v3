# 03 — Render the six-face planet surface

Status: ready-for-agent
Blocked by: 01, 02

## Scope
Generate the low-resolution (24×24 per face) surface mesh from logical tiles and render six distinct face debug colors. No biomes, resources, or terrain noise.

## Acceptance criteria
- One planet renders with all six faces, no loading transitions.
- Mesh vertices use cube-to-sphere projection; face identity remains inspectable.
- Face colors can be toggled for seam diagnosis.

## Tests
- Mesh generation tests assert expected tile/face counts and valid indices.
- Run a scene and inspect that all six faces render.