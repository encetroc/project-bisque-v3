# 23 — Place ceramic inventory objects on the planet

Status: ready-for-agent
Blocked by: 07, 14, 16, 22

## Scope
Select a ceramic from inventory, show surface-normal-aligned placement ghost, rotate around hit normal, confirm placement or cancel. No pickup workflow.

## Acceptance criteria
- Camera ray hit determines valid surface location and local up.
- Ghost tracks surface, rotates around hit normal, and confirms/cancels via controls.
- Inventory removal happens only on successful placement; placed object is static and saved-ready data.

## Tests
- Tests cover surface orientation and inventory transaction; manual placement around sphere.