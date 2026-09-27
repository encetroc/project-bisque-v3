# 13 — Place physical resource nodes by biome

Status: ready-for-agent
Blocked by: 12, 07

## Scope
Add simple clay, wood, mineral, shell/sand resource node entities and deterministic authored/spawn placement appropriate to their regions. No gathering or inventory mutation.

## Acceptance criteria
- Meadow has common clay/wood/plants; Highlands red clay/iron; Coast pale clay/shells/sand.
- Nodes sit on the spherical surface and have the primitive visual language from spec.
- Deep water contains no walkable resource nodes.

## Tests
- Spawn-data tests verify biome/type compatibility; inspect placement/orientation in game.