# 28 — Run deterministic NPC schedules on waypoints

Status: ready-for-agent
Blocked by: 17, 27

## Scope
Implement authored waypoint paths and schedule state transitions Home/Travel/Work/Social/Travel/Home for the three NPCs. No general pathfinding.

## Acceptance criteria
- Same time/day produces deterministic state and destination.
- NPC follows authored planet-projected waypoints; no arbitrary planetary pathfinding.
- Schedule debug state/destination is inspectable.

## Tests
- Clock-boundary tests for schedule states; manual circuit and seam-adjacent waypoint check.