use std::sync::Arc;

use jiff::tz::TimeZone as JiffTimeZone;

use crate::{
    LoadedEntry, PricingMap, TimestampMs, TokenUsageRaw, UsageEntry, UsageMessage,
    calculate_cost_for_usage, cli::CostMode, format_date_tz, missing_pricing_model_for_candidates,
    total_usage_tokens,
};

pub(super) const SCHEMA_VERSION: i64 = 1;

pub(super) struct RhoUsageEvent {
    pub(super) event_id: String,
    pub(super) schema_version: i64,
    pub(super) occurred_at_ms: i64,
    pub(super) session_id: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) workspace_path: Option<String>,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) input_tokens: Option<i64>,
    pub(super) output_tokens: Option<i64>,
    pub(super) cache_read_tokens: Option<i64>,
    pub(super) cache_write_tokens: Option<i64>,
    pub(super) total_tokens: Option<i64>,
    pub(super) cost_usd_micros: Option<i64>,
    pub(super) rho_version: Option<String>,
}

pub(super) fn event_to_entry(
    event: RhoUsageEvent,
    tz: Option<&JiffTimeZone>,
    mode: CostMode,
    pricing: &PricingMap,
) -> Option<LoadedEntry> {
    if event.schema_version != SCHEMA_VERSION || event.occurred_at_ms <= 0 {
        return None;
    }
    let input_tokens = non_negative(event.input_tokens);
    let output_tokens = non_negative(event.output_tokens);
    let cache_read_tokens = non_negative(event.cache_read_tokens);
    let cache_write_tokens = non_negative(event.cache_write_tokens);
    let usage = TokenUsageRaw {
        input_tokens,
        output_tokens,
        cache_creation_input_tokens: cache_write_tokens,
        cache_read_input_tokens: cache_read_tokens,
        speed: None,
        cache_creation: None,
    };
    let total_tokens = non_negative(event.total_tokens);
    let extra_total_tokens = total_tokens.saturating_sub(total_usage_tokens(usage));
    if total_usage_tokens(usage) == 0 && extra_total_tokens == 0 {
        return None;
    }

    let timestamp = TimestampMs::from_millis(event.occurred_at_ms);
    let session_id = event
        .session_id
        .filter(|value| !value.is_empty())
        .or_else(|| event.run_id.filter(|value| !value.is_empty()))
        .unwrap_or_else(|| event.event_id.clone());
    let project_path = event
        .workspace_path
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Rho".to_string());
    let cost_usd = event
        .cost_usd_micros
        .and_then(|value| (value >= 0).then_some(value as f64 / 1_000_000.0));
    let candidates = model_candidates(&event.model, &event.provider);
    let pricing_model = candidates
        .iter()
        .find(|candidate| pricing.find(candidate).is_some())
        .map(String::as_str)
        .unwrap_or(event.model.as_str());
    let cost = calculate_cost_for_usage(Some(pricing_model), usage, cost_usd, mode, Some(pricing));
    let missing_pricing_model = match mode {
        CostMode::Display => None,
        CostMode::Auto if cost_usd.is_some() => None,
        CostMode::Auto | CostMode::Calculate => missing_pricing_model_for_candidates(
            &event.model,
            candidates,
            total_usage_tokens(usage),
            Some(pricing),
        ),
    };
    let timestamp_text = crate::format_rfc3339_millis(timestamp);
    let data = UsageEntry {
        session_id: Some(session_id.clone()),
        timestamp: timestamp_text,
        version: event.rho_version,
        message: UsageMessage {
            usage,
            model: Some(event.model.clone()),
            id: Some(event.event_id),
        },
        cost_usd,
        request_id: None,
        is_api_error_message: None,
        is_sidechain: None,
    };

    Some(LoadedEntry {
        data,
        timestamp,
        date: format_date_tz(timestamp, tz),
        project: Arc::from("rho"),
        session_id: Arc::from(session_id),
        project_path: Arc::from(project_path),
        cost,
        extra_total_tokens,
        credits: None,
        message_count: None,
        model: Some(event.model),
        usage_limit_reset_time: None,
        missing_pricing_model,
    })
}

fn non_negative(value: Option<i64>) -> u64 {
    value
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0)
}

fn model_candidates(model: &str, provider: &str) -> Vec<String> {
    let provider = provider.replace('-', "_");
    let mut candidates = Vec::with_capacity(2);
    if !provider.is_empty() && provider != "unknown" && provider != "rho" {
        candidates.push(format!("{provider}/{model}"));
    }
    candidates.push(model.to_string());
    candidates.dedup();
    candidates
}
