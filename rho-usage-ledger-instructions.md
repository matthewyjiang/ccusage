# Add a durable usage ledger to Rho

## Goal

Persist provider-neutral model usage in Rho so external reporting tools can read accurate accounting data. Cover interactive sessions, automation, delegated agents, compaction, title generation, retries, failures, and cancellation.

This work belongs entirely in the Rho repository. Do not add reporting or aggregation commands to Rho as part of this change.

## Existing usage model

Use `ModelUsage` from `crates/rho-sdk/src/model.rs` as the accounting source. Preserve its fields without deriving usage from transcripts, session snapshots, or delegated-run `result.json` files:

- uncached input tokens
- output tokens
- cache-read tokens
- cache-write tokens
- total tokens
- provider-reported cost in USD micros

## Storage

Create a dedicated SQLite database:

```text
${RHO_HOME:-~/.rho}/usage.sqlite3
```

Add `RHO_HOME` as the Rho data-root override if it does not already exist. Do not put accounting data in `~/.rho/sessions/index.sqlite3`; that index is rebuildable, while usage must be durable.

Use Rho's existing `rusqlite` dependency. Configure the database with WAL mode, a reasonable busy timeout, restrictive permissions, explicit migrations, and `PRAGMA user_version`.

## Event boundary

Write one immutable row for each provider request. Do not persist one cumulative row per session, user turn, or `RunOutcome`.

Accumulate streamed `ModelEvent::Usage` values for one request and write the row when the request terminates. Preserve observed usage from failed or cancelled requests. Record separately billed retries as separate events.

Put the recorder at the orchestration or provider-request boundary, not in TUI rendering or event-consumer code. Make all model-backed paths use the same recorder:

- interactive turns
- automation
- foreground and background delegated agents
- explicit and automatic compaction
- title generation
- retries
- provider failures
- cancellation after usage was reported

## Version 1 schema

Use a schema equivalent to:

```sql
CREATE TABLE usage_events (
    event_id           TEXT PRIMARY KEY,
    schema_version     INTEGER NOT NULL,
    occurred_at_ms     INTEGER NOT NULL,

    session_id         TEXT,
    parent_session_id  TEXT,
    run_id             TEXT,
    step_index         INTEGER,
    attempt_index      INTEGER,
    workspace_path     TEXT,

    provider           TEXT NOT NULL,
    model              TEXT NOT NULL,
    purpose            TEXT NOT NULL,

    input_tokens       INTEGER,
    output_tokens      INTEGER,
    cache_read_tokens  INTEGER,
    cache_write_tokens INTEGER,
    total_tokens       INTEGER,
    cost_usd_micros    INTEGER,

    rho_version        TEXT
);

CREATE INDEX usage_events_occurred_at
    ON usage_events (occurred_at_ms);

CREATE INDEX usage_events_session
    ON usage_events (session_id, occurred_at_ms);
```

Store timestamps as Unix milliseconds in UTC. Keep unavailable token values as `NULL`; an omitted provider field is not the same as a reported zero. Store raw provider and model identities without translating them into pricing-library aliases.

Use string purpose values so new purposes remain additive. Start with:

- `agent`
- `subagent`
- `compaction`
- `title`

Reject or safely clamp Rust `u64` values that cannot fit in SQLite's signed 64-bit integer range.

## Identity and idempotency

Give every provider request a stable `event_id`. Prefer an identity derived from request identity, such as `run_id + step_index + attempt_index + request identifier`. A UUID is acceptable if the recorder creates it once and retains it across write retries.

Use an idempotent insertion such as `INSERT OR IGNORE`. Do not derive identity only from timestamps, session IDs, or token counts. Keep actual provider retries separate by including the attempt or request identity.

## Cost semantics

Persist `ModelUsage::cost_usd_micros` only when the provider supplied it. Leave the field `NULL` when Rho has only a local estimate. If Rho later persists estimates, add an explicit cost-source field first.

Preserve each raw token category even when the provider also reports `total_tokens`. Do not redistribute unexplained total tokens into input or output.

## Reliability

Use a short transaction after each provider request. In async code, put SQLite work behind the appropriate blocking boundary. Avoid a fire-and-forget queue that can lose pending events during normal process exit.

A ledger-write failure must not discard an otherwise successful provider response. Return the model result and emit a bounded diagnostic without prompts, credentials, reasoning, tool arguments, or provider payloads.

External readers will open the database read-only while Rho is running. Test concurrent writers and a live read-only connection under WAL mode.

## Compatibility and privacy

Treat the database as a stable external-read contract:

- version it with `PRAGMA user_version`
- retain row `schema_version`
- prefer additive migrations
- preserve old columns during migrations
- document units, nullability, timestamp conventions, and purpose values
- provide a small sanitized fixture database
- never store prompts, responses, reasoning, tool arguments, credentials, or raw provider payloads

## Suggested structure

A suitable application-level layout is:

```text
crates/rho/src/usage/
├── mod.rs
├── event.rs
├── recorder.rs
├── sqlite.rs
└── migrations.rs
```

Follow existing Rho module conventions if another location fits better. Keep SQL and migrations out of orchestration modules. Orchestration should create a typed event and call a narrow recorder interface that tests can replace.

## Tests

Add tests for:

1. New database creation and private permissions.
2. Every supported schema migration.
3. Exact token-category and USD-micros persistence.
4. `NULL` preservation for unavailable fields.
5. Safe `u64` to SQLite integer conversion.
6. Provider, model, purpose, timestamp, session, run, and workspace metadata.
7. Multiple requests in one run.
8. Separately billed retries.
9. Idempotent write retries.
10. Usage observed before failure or cancellation.
11. Interactive, automation, delegated, compaction, and title-model paths.
12. Concurrent writers and a read-only client.
13. Lock contention and write-failure behavior.
14. Absence of transcript and secret material.

Use Rho's deterministic provider fixtures and assert exact stored rows.

## Acceptance criteria

The change is complete when Rho writes one durable row per provider request across every model-backed path, preserves cache categories without double-counting, records observed failed and cancelled usage, keeps separately billed retries distinct, supports concurrent processes and live readers, passes migration tests, and stores accounting metadata only. Existing session and delegated-run status behavior must remain unchanged.
