use std::path::Path;

use crate::{LoadedEntry, PricingMap, Result, cli::SharedArgs, debug_log, parse_tz};

use super::{
    parser::{RhoUsageEvent, event_to_entry},
    paths::db_path,
};

const USAGE_QUERY: &str = "SELECT
    event_id, schema_version, occurred_at_ms, session_id, run_id, workspace_path,
    provider, model, input_tokens, output_tokens, cache_read_tokens,
    cache_write_tokens, total_tokens, cost_usd_micros, rho_version
    FROM usage_events ORDER BY occurred_at_ms, event_id";

pub(crate) fn load_entries(shared: &SharedArgs, pricing: &PricingMap) -> Result<Vec<LoadedEntry>> {
    crate::progress::track_usage_load(crate::progress::UsageLoadAgent::Rho, shared.json, || {
        Ok(load_entries_inner(shared, pricing))
    })
}

fn load_entries_inner(shared: &SharedArgs, pricing: &PricingMap) -> Vec<LoadedEntry> {
    let Some(path) = db_path() else {
        return Vec::new();
    };
    load_entries_from_database(&path, shared, pricing)
}

fn load_entries_from_database(
    path: &Path,
    shared: &SharedArgs,
    pricing: &PricingMap,
) -> Vec<LoadedEntry> {
    let Ok(connection) =
        sqlite::Connection::open_with_flags(path, sqlite::OpenFlags::new().with_read_only())
    else {
        debug_log(
            shared,
            format!("Failed to open Rho usage database: {}", path.display()),
        );
        return Vec::new();
    };
    let Ok(mut statement) = connection.prepare(USAGE_QUERY) else {
        debug_log(
            shared,
            format!("Failed to read Rho usage database: {}", path.display()),
        );
        return Vec::new();
    };
    let tz = parse_tz(shared.timezone.as_deref());
    let mut entries = Vec::new();
    loop {
        match statement.next() {
            Ok(sqlite::State::Row) => {
                let event = RhoUsageEvent {
                    event_id: match statement.read(0) {
                        Ok(value) => value,
                        Err(_) => continue,
                    },
                    schema_version: match statement.read(1) {
                        Ok(value) => value,
                        Err(_) => continue,
                    },
                    occurred_at_ms: match statement.read(2) {
                        Ok(value) => value,
                        Err(_) => continue,
                    },
                    session_id: statement.read(3).ok().flatten(),
                    run_id: statement.read(4).ok().flatten(),
                    workspace_path: statement.read(5).ok().flatten(),
                    provider: match statement.read(6) {
                        Ok(value) => value,
                        Err(_) => continue,
                    },
                    model: match statement.read(7) {
                        Ok(value) => value,
                        Err(_) => continue,
                    },
                    input_tokens: statement.read(8).ok().flatten(),
                    output_tokens: statement.read(9).ok().flatten(),
                    cache_read_tokens: statement.read(10).ok().flatten(),
                    cache_write_tokens: statement.read(11).ok().flatten(),
                    total_tokens: statement.read(12).ok().flatten(),
                    cost_usd_micros: statement.read(13).ok().flatten(),
                    rho_version: statement.read(14).ok().flatten(),
                };
                if let Some(entry) = event_to_entry(event, tz.as_ref(), shared.mode, pricing) {
                    entries.push(entry);
                }
            }
            Ok(sqlite::State::Done) => break,
            Err(_) => {
                debug_log(
                    shared,
                    format!("Failed to query Rho usage database: {}", path.display()),
                );
                break;
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ccusage_test_support::{EnvVarGuard, fs_fixture};

    use crate::{
        PricingMap,
        cli::{CostMode, SharedArgs},
    };

    fn create_usage_database(path: &Path) {
        let db = sqlite::open(path).unwrap();
        db.execute(
            "CREATE TABLE usage_events (
                event_id TEXT PRIMARY KEY,
                schema_version INTEGER NOT NULL,
                occurred_at_ms INTEGER NOT NULL,
                session_id TEXT,
                parent_session_id TEXT,
                run_id TEXT,
                step_index INTEGER,
                attempt_index INTEGER,
                workspace_path TEXT,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                purpose TEXT NOT NULL,
                input_tokens INTEGER,
                output_tokens INTEGER,
                cache_read_tokens INTEGER,
                cache_write_tokens INTEGER,
                total_tokens INTEGER,
                cost_usd_micros INTEGER,
                rho_version TEXT
            )",
        )
        .unwrap();
        db.execute(
            "INSERT INTO usage_events VALUES (
                'event-1', 1, 1767312000000, 'session-a', NULL, 'run-a', 0, 0,
                '/workspace/project', 'anthropic', 'claude-sonnet-4-20250514', 'agent',
                100, 50, 10, 20, 185, 20000, '0.9.0'
            )",
        )
        .unwrap();
    }

    #[test]
    fn loads_rho_usage_events_from_sqlite() {
        let fixture = fs_fixture!({});
        create_usage_database(&fixture.path("usage.sqlite3"));
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());
        let shared = SharedArgs {
            mode: CostMode::Display,
            timezone: Some("UTC".to_string()),
            ..SharedArgs::default()
        };

        let entries = super::load_entries(&shared, &PricingMap::load_embedded()).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].date, "2026-01-02");
        assert_eq!(entries[0].session_id.as_ref(), "session-a");
        assert_eq!(entries[0].project_path.as_ref(), "/workspace/project");
        assert_eq!(
            entries[0].model.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(entries[0].data.message.usage.input_tokens, 100);
        assert_eq!(entries[0].data.message.usage.output_tokens, 50);
        assert_eq!(entries[0].data.message.usage.cache_read_input_tokens, 10);
        assert_eq!(
            entries[0].data.message.usage.cache_creation_input_tokens,
            20
        );
        assert_eq!(entries[0].extra_total_tokens, 5);
        assert_eq!(entries[0].cost, 0.02);
        assert_eq!(entries[0].data.version.as_deref(), Some("0.9.0"));
    }

    #[test]
    fn session_summary_preserves_workspace_and_activity_metadata() {
        let fixture = fs_fixture!({});
        create_usage_database(&fixture.path("usage.sqlite3"));
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());
        let entries =
            super::load_entries(&SharedArgs::default(), &PricingMap::load_embedded()).unwrap();

        let rows = super::super::summarize_entries(&entries, crate::cli::AgentReportKind::Session)
            .unwrap();

        assert_eq!(rows[0].session_id.as_deref(), Some("session-a"));
        assert_eq!(rows[0].project_path.as_deref(), Some("/workspace/project"));
        assert_eq!(
            rows[0].last_activity.as_deref(),
            Some("2026-01-02T00:00:00.000Z")
        );
    }

    #[test]
    fn auto_mode_prefers_provider_reported_cost() {
        let fixture = fs_fixture!({});
        create_usage_database(&fixture.path("usage.sqlite3"));
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());
        let shared = SharedArgs {
            mode: CostMode::Auto,
            ..SharedArgs::default()
        };

        let entries = super::load_entries(&shared, &PricingMap::load_embedded()).unwrap();

        assert_eq!(entries[0].cost, 0.02);
    }

    #[test]
    fn calculate_mode_ignores_provider_reported_cost() {
        let fixture = fs_fixture!({});
        create_usage_database(&fixture.path("usage.sqlite3"));
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());
        let shared = SharedArgs {
            mode: CostMode::Calculate,
            ..SharedArgs::default()
        };

        let entries = super::load_entries(&shared, &PricingMap::load_embedded()).unwrap();

        assert!(entries[0].cost > 0.0);
        assert_ne!(entries[0].cost, 0.02);
    }

    #[test]
    fn keeps_unclassified_total_tokens_out_of_output_usage() {
        let fixture = fs_fixture!({});
        let path = fixture.path("usage.sqlite3");
        create_usage_database(&path);
        sqlite::open(&path)
            .unwrap()
            .execute(
                "UPDATE usage_events SET input_tokens = NULL, output_tokens = NULL,
                 cache_read_tokens = NULL, cache_write_tokens = NULL, total_tokens = 25",
            )
            .unwrap();
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());

        let entries =
            super::load_entries(&SharedArgs::default(), &PricingMap::load_embedded()).unwrap();

        assert_eq!(entries[0].data.message.usage.output_tokens, 0);
        assert_eq!(entries[0].extra_total_tokens, 25);
    }

    #[test]
    fn skips_events_from_unsupported_schema_versions() {
        let fixture = fs_fixture!({});
        let path = fixture.path("usage.sqlite3");
        create_usage_database(&path);
        sqlite::open(&path)
            .unwrap()
            .execute("UPDATE usage_events SET schema_version = 2")
            .unwrap();
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());

        let entries =
            super::load_entries(&SharedArgs::default(), &PricingMap::load_embedded()).unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn falls_back_to_run_and_event_identity_when_session_metadata_is_missing() {
        let fixture = fs_fixture!({});
        let path = fixture.path("usage.sqlite3");
        create_usage_database(&path);
        sqlite::open(&path)
            .unwrap()
            .execute(
                "UPDATE usage_events SET session_id = NULL, workspace_path = NULL, total_tokens = NULL",
            )
            .unwrap();
        let _rho_home = EnvVarGuard::set("RHO_HOME", fixture.root());

        let entries =
            super::load_entries(&SharedArgs::default(), &PricingMap::load_embedded()).unwrap();

        assert_eq!(entries[0].session_id.as_ref(), "run-a");
        assert_eq!(entries[0].project_path.as_ref(), "Rho");
        assert_eq!(entries[0].extra_total_tokens, 0);
    }
}
