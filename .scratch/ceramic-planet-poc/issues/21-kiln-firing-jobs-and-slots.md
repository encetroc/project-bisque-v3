# 21 — Fire dry objects in the kiln

Status: ready-for-agent
Blocked by: 17, 18, 20

## Scope
Implement base kiln with two slots; fire dry ceramics for four game hours into fired ceramic items. Glaze treatment is carried through firing.

## Acceptance criteria
- Only dry objects enter the kiln; completion yields fired state.
- Jobs use game time and preserve item identity/material/glaze.
- Full kiln rejects additional objects safely.

## Tests
- Integration tests for invalid inputs, exact completion boundary, slots, and fired output.