# 14 — Implement slot and stack inventory data

Status: ready-for-agent
Blocked by: none

## Scope
Create 20-slot inventory data model with stackable resource counts and one-slot ceramic objects. UI is out of scope.

## Acceptance criteria
- Adding items stacks compatible resources, respects capacity, and reports remainder.
- Unique ceramic objects occupy individual slots.
- Inventory can be queried/removed safely.

## Tests
- Unit tests cover stack merge, full inventory, removal, and single-object slots.