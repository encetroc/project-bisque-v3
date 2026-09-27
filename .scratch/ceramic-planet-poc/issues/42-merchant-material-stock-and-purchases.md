# 42 — Sell basic materials and buy merchant stock

Status: ready-for-agent
Blocked by: 14, 26, 27

## Scope
Add fixed merchant stock/prices for basic materials and upgrade items, with player purchase transactions. Keep economy fixed-price and small.

## Acceptance criteria
- Purchases require sufficient coins and inventory capacity and apply atomically.
- Merchant stock is explicit and limited to specified basic/upgrade items.
- Selling resources and fired ceramics uses fixed prices; no dynamic economy.

## Tests
- Transaction tests for insufficient coins/space, valid purchase, stock boundaries, and valid sale.