# 20 — Dry greenware on the rack

Status: ready-for-agent
Blocked by: 17, 18, 19

## Scope
Implement base rack with two slots; insert/remove greenware and advance drying by four game hours. Objects visibly occupy slots.

## Acceptance criteria
- Only greenware enters available slots.
- At four game hours, item state becomes dry; earlier it remains greenware.
- Full rack rejects extra items without consuming them.

## Tests
- Integration tests for capacity, elapsed-time boundary, and output stage.