# Rho adapter

The Rho adapter reads provider-neutral usage events from Rho's SQLite usage ledger.

## Commands

```sh
ccusage rho daily
ccusage rho weekly
ccusage rho monthly
ccusage rho session
```

Rho usage is also included in unified reports such as `ccusage daily` when the database contains supported events.

## Data location

The adapter reads:

```text
~/.rho/usage.sqlite3
```

Set `RHO_HOME` to override the `~/.rho` directory:

```sh
RHO_HOME=/path/to/rho ccusage rho daily
```

The adapter opens the database read-only, so it can report usage while Rho is running.

## Supported schema

The initial adapter supports usage event schema version 1. It reads one row per provider request from the `usage_events` table. The required accounting fields are:

- `event_id`
- `schema_version`
- `occurred_at_ms`
- `provider`
- `model`
- `input_tokens`
- `output_tokens`
- `cache_read_tokens`
- `cache_write_tokens`
- `total_tokens`
- `cost_usd_micros`

It also reads `session_id`, `run_id`, `workspace_path`, and `rho_version` for report metadata. Unknown columns are ignored. Rows with unsupported schema versions, invalid timestamps, or no token usage are skipped.

Token fields use these mappings:

| Rho field | ccusage field |
| --- | --- |
| `input_tokens` | Input tokens |
| `output_tokens` | Output tokens |
| `cache_read_tokens` | Cache read tokens |
| `cache_write_tokens` | Cache creation tokens |

When `total_tokens` exceeds the sum of known token categories, ccusage reports the difference as extra total tokens. It does not reclassify the difference as output.

Session grouping uses `session_id`, then `run_id`, then `event_id` as fallbacks. A missing workspace path appears as `Rho`.

## Cost behavior

Rho stores provider-reported cost as integer USD micros. ccusage converts that value to USD and applies its standard cost modes:

- `auto` uses the stored cost when present and otherwise calculates from tokens.
- `calculate` ignores the stored cost and calculates from tokens.
- `display` uses the stored cost and shows zero when it is absent.

For calculated costs, the adapter tries a provider-qualified model name before the raw model name. Rho's stored provider and model values remain unchanged in report output.
