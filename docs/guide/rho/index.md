# Rho data source (experimental)

> Rho support is experimental and requires a [Rho](https://github.com/matthewyjiang/rho) build that writes usage ledger schema version 1.

ccusage reads Rho's provider-neutral SQLite usage ledger and supports both focused and unified reports.

## Focused views

::: code-group

```bash [bunx (Recommended)]
bunx ccusage rho --help
```

```bash [npx]
npx ccusage@latest rho --help
```

```bash [pnpm]
pnpm dlx ccusage rho --help
```

:::

## Data source

The CLI reads `usage.sqlite3` from `RHO_HOME`, which defaults to `~/.rho`:

```bash
RHO_HOME=/path/to/rho ccusage rho daily
```

```text
~/.rho/
└── usage.sqlite3
```

ccusage opens the database read-only, so you can run reports while Rho is active. Unified reports detect Rho when the database contains supported usage events.

## Report views

| Focused view          | Description                | See also                                |
| --------------------- | -------------------------- | --------------------------------------- |
| `ccusage rho daily`   | Aggregate usage by date    | [Daily Usage](/guide/daily-reports)     |
| `ccusage rho weekly`  | Aggregate usage by week    | [Weekly Usage](/guide/weekly-reports)   |
| `ccusage rho monthly` | Aggregate usage by month   | [Monthly Usage](/guide/monthly-reports) |
| `ccusage rho session` | Group usage by Rho session | [Session Usage](/guide/session-reports) |

These views support the shared date, timezone, JSON, pricing, sorting, breakdown, and compact-output options.

## Usage mapping

Rho stores one accounting event per provider request. ccusage maps its fields as follows:

- `input_tokens` becomes uncached input usage.
- `output_tokens` becomes output usage.
- `cache_read_tokens` becomes cache-read usage.
- `cache_write_tokens` becomes cache-creation usage.
- Token counts present only in `total_tokens` remain extra total tokens.
- `cost_usd_micros` becomes provider-reported cost without a floating-point storage conversion.

Session reports group by `session_id`, then fall back to `run_id` or `event_id`. A missing workspace path appears as `Rho`.

## Cost modes

- `--mode auto` uses Rho's provider-reported cost when present and otherwise calculates from token counts.
- `--mode calculate` ignores the stored cost and calculates from token counts.
- `--mode display` uses the stored cost and shows zero when it is absent.

Calculated costs try the provider-qualified model name before the raw model name.

## Environment variables

| Variable    | Description                                           |
| ----------- | ----------------------------------------------------- |
| `RHO_HOME`  | Override the Rho data root containing `usage.sqlite3` |
| `LOG_LEVEL` | Adjust verbosity (0 silent through 5 trace)           |

## Troubleshooting

::: details No Rho usage data found
Confirm that `$RHO_HOME/usage.sqlite3` exists and contains `usage_events` rows with `schema_version = 1`. Older Rho builds do not persist the usage ledger required by this adapter.
:::

::: details Some events are missing
ccusage skips rows with unsupported schema versions, invalid timestamps, or no token usage. Run with `LOG_LEVEL=4` to see database access diagnostics.
:::

::: details Costs show as $0.00
Use `--mode auto` or `--mode display` when Rho recorded provider cost. For calculated costs, confirm that LiteLLM recognizes the stored provider and model combination.
:::
