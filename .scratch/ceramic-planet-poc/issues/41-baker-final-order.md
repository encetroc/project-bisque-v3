# 41 — Add the Baker’s final bakery-upgrade order

Status: ready-for-agent
Blocked by: 30, 31, 32, 35

## Scope
Add final authored Baker request for four fired cups, two fired bowls, and one fired vase; completion triggers the single bakery upgrade and assigns the delivered objects to its predefined slots.

## Acceptance criteria
- Requirements are exact; vase remains gated by the specified progression.
- Delivery consumes the correct fired objects and cannot be claimed twice.
- Bakery upgrade and visible objects appear only after successful completion.

## Tests
- Integration tests for missing items, valid completion, repeated claim, and post-upgrade slot contents.