---
title: Pipelines
order: 7
---

# Chapter 7: Pipelines

**Atomic multi-step operations: the equivalent of stored procedures.**

---

## What is a Pipeline?

A pipeline is an ordered list of steps executed atomically against your database. If any step fails (e.g., a validation), all previous mutations are rolled back. This is how BitBin implements transactions, constraints, and business logic.

---

## Pipeline Steps

### Scan

Execute a query and store the result in the pipeline context.

```json
{"Scan": {"name": "high-value", "query": {"space": {"amount": [500, null]}, "measure": {"type": "count"}}}}
```

### Lookup

Find a record by multi-predicate seek. Stores the found slot in context under `name`.

```json
{"Lookup": {"name": "sender", "predicates": [[0, 2, 4, 1]]}}
```

Predicate format: `[col_offset, col_width, value_width, value]`
- `col_offset`: byte offset in the 16-byte record
- `col_width`: width of the column in bytes
- `value_width`: width of the search value
- `value`: the value to match

### ValidateGte

Assert that a field at a looked-up slot is ≥ a minimum value. If the assertion fails, the entire pipeline rolls back.

```json
{"ValidateGte": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "min_value": 3000}}
```

This is the equivalent of a CHECK constraint — but programmable.

### ValidateEq

Assert that a field equals an exact value.

```json
{"ValidateEq": {"slot_ref": "sender", "col_offset": 11, "col_width": 1, "expected": 1}}
```

### UpdateDelta

Add a delta (positive or negative) to a field at a looked-up slot.

```json
{"UpdateDelta": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "delta": -3000}}
```

### SetField

Set a field to an absolute value.

```json
{"SetField": {"slot_ref": "sender", "col_offset": 11, "col_width": 1, "value": 2}}
```

### Insert

Insert a new record.

```json
{"Insert": {"tenant": 1, "entity": 0, "key_id": 100, "amount": 5000, "region": 3, "currency": 1, "category": 5, "status": 0}}
```

### Retract

Delete a looked-up record.

```json
{"Retract": {"slot_ref": "sender"}}
```

### Notify

Signal reactive subscribers that data has changed.

```json
"Notify"
```

---

## Example: Bank Transfer

Transfer 3000 from sender (key_id=1) to receiver (key_id=2), with balance validation:

```json
{
  "slug": "bank-transfer",
  "trigger": "on_call",
  "steps": [
    {"Lookup": {"name": "sender", "predicates": [[0, 2, 4, 1]]}},
    {"Lookup": {"name": "receiver", "predicates": [[0, 2, 4, 2]]}},
    {"ValidateGte": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "min_value": 3000}},
    {"UpdateDelta": {"slot_ref": "sender", "col_offset": 6, "col_width": 2, "delta": -3000}},
    {"UpdateDelta": {"slot_ref": "receiver", "col_offset": 6, "col_width": 2, "delta": 3000}},
    "Notify"
  ],
  "description": "Transfer 3000 from sender to receiver with balance check"
}
```

**Execution flow:**
1. Look up sender by key_id=1 → found at slot X
2. Look up receiver by key_id=2 → found at slot Y
3. Check sender.amount ≥ 3000 → if not, **rollback** (no mutation applied)
4. Subtract 3000 from sender.amount
5. Add 3000 to receiver.amount
6. Notify subscribers

If step 3 fails, steps 4 and 5 never execute. All lookups are captured with old values for rollback.

---

## Triggers

### on_call

Execute only when explicitly called via `POST /pipeline/:slug/call`.

```json
{"trigger": "on_call"}
```

### on_insert

Fire automatically when a record is inserted that matches the filter criteria.

```json
{
  "trigger": "on_insert",
  "entity_filter": 0,
  "tenant_filter": 1
}
```

This fires for every insert where entity=0 and tenant=1.

### on_schedule

Fire on a time interval.

```json
{
  "trigger": "on_schedule",
  "schedule_interval_secs": 3600
}
```

---

## Keyed CRUD

These are the low-level record operations that pipelines build on:

| Operation | Description | Complexity |
|---|---|---|
| `keyed_insert(tenant, entity, key_id, ...)` | Upsert by primary key | O(1) amortized |
| `keyed_lookup(tenant, entity, key_id)` | Point read by primary key | O(1) |
| `keyed_delete(tenant, entity, key_id)` | Delete by primary key | O(1) |

All three use the composite key `(tenant, entity, key_id)` to locate the record slot.

---

[← Previous: WebSocket Protocol](/websocket) · **Chapter 7** · [Next: Examples & Recipes →](/examples)
