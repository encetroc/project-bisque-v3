# 31 — Populate NPC property slots with delivered ceramics

Status: ready-for-agent
Blocked by: 07, 22, 23, 30

## Scope
Add predefined placement slots with allowed categories and occupancy, then place some delivered Baker cups into bakery slots as persistent world entities.

## Acceptance criteria
- Slot validates allowed object categories and single occupancy.
- Completed request assigns cups to deterministic slots and renders them in bakery.
- Objects remain there on later visits; counts do not produce unlimited clutter.

## Tests
- Unit tests for slot eligibility/capacity and integration test delivery → visible cups.