# 33 — Add one upgrade per studio machine

Status: ready-for-agent
Blocked by: 19, 20, 21, 26

## Scope
Add exactly one purchasable upgrade each: workbench unlocks vase, rack grows 2→4 slots, kiln grows 2→4 slots. Require configured coins/materials.

## Acceptance criteria
- Each machine upgrades once and effects match its stated benefit.
- Insufficient payment fails atomically; no further upgrade tier exists.
- Upgrades are independently usable through machine interaction.

## Tests
- Unit/integration tests for each cost/effect and repeat/insufficient payment.