use serde::Serialize;
use tauri::State;

use crate::state::TauriState;

#[derive(Debug, Clone, Serialize)]
pub struct ThreadCost {
    pub thread_id: String,
    pub thread_title: String,
    pub cost_usd: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCost {
    pub model: String,
    pub cost_usd: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub total_cost_usd: f64,
    pub thread_costs: Vec<ThreadCost>,
    pub model_costs: Vec<ModelCost>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadUsage {
    pub thread_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub cost_usd: f64,
}

#[tauri::command]
pub fn get_usage_summary(state: State<'_, TauriState>) -> Result<UsageSummary, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let conn = db.conn();

    let (total_input, total_output, total_cache_read, total_cache_write, total_cost) =
        codeforge_persistence::queries::get_usage_totals(conn).map_err(|e| e.to_string())?;

    let thread_rows =
        codeforge_persistence::queries::get_usage_by_thread(conn).map_err(|e| e.to_string())?;

    let thread_costs: Vec<ThreadCost> = thread_rows
        .into_iter()
        .map(|(tid, cost, tokens)| {
            let title = uuid::Uuid::parse_str(&tid)
                .ok()
                .and_then(|uuid| {
                    codeforge_persistence::queries::get_thread_by_id(conn, uuid)
                        .ok()
                        .flatten()
                })
                .map(|t| t.title)
                .unwrap_or_else(|| tid.clone());
            ThreadCost {
                thread_id: tid,
                thread_title: title,
                cost_usd: cost,
                total_tokens: tokens,
            }
        })
        .collect();

    let model_rows =
        codeforge_persistence::queries::get_usage_by_model(conn).map_err(|e| e.to_string())?;

    let model_costs: Vec<ModelCost> = model_rows
        .into_iter()
        .map(|(model, cost, tokens)| ModelCost {
            model,
            cost_usd: cost,
            total_tokens: tokens,
        })
        .collect();

    Ok(UsageSummary {
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_cache_read_tokens: total_cache_read,
        total_cache_write_tokens: total_cache_write,
        total_cost_usd: total_cost,
        thread_costs,
        model_costs,
    })
}

#[tauri::command]
pub fn get_thread_usage(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<ThreadUsage, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let (input, output, cache_read, cache_write, cost) =
        codeforge_persistence::queries::get_usage_for_thread(db.conn(), &thread_id)
            .map_err(|e| e.to_string())?;

    Ok(ThreadUsage {
        thread_id,
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
        cost_usd: cost,
    })
}
