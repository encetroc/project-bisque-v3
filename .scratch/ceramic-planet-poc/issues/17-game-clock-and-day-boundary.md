# 17 — Add game clock and day transition event

Status: ready-for-agent
Blocked by: none

## Scope
Implement clock at 1 real second = 6 game minutes, current day/time, sleep-to-next-morning action, and a single day-boundary event for subscribers.

## Acceptance criteria
- Time advances at configured rate and rolls days predictably.
- Sleep advances directly to next morning.
- Consumers receive exactly one transition event per crossed day.

## Tests
- Unit tests for rate conversion, rollover, sleep, and event count.