# 38 — Persist core game state in one save slot

Status: ready-for-agent
Blocked by: 14, 17, 20, 21, 23, 28, 30, 31, 32, 33, 34

## Scope
Serialize/restore player surface position, inventory, coins, time/day, machine jobs/upgrades, NPC friendship/requests/upgrades, gathered resources, player placements, and NPC slot occupancy. One save slot only.

## Acceptance criteria
- Save then load restores all listed state without duplicates or lost objects.
- Save format handles absent/invalid file by starting a clean game safely.
- No generalized save framework or multiple slots.

## Tests
- Round-trip tests cover each state category and malformed/absent save behavior.