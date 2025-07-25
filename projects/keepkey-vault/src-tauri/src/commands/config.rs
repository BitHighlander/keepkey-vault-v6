// commands/config.rs - Configuration and onboarding commands

use std::sync::Arc;
use tauri::State;
use keepkey_db::Database;

/// Check if this is the first time the app is being installed/run
#[tauri::command]
pub async fn is_first_time_install(
    database: State<'_, Arc<Database>>,
) -> Result<bool, String> {
    match database.is_first_time_install().await {
        Ok(is_first_time) => Ok(is_first_time),
        Err(e) => {
            log::error!("Failed to check first time install status: {}", e);
            Err(format!("Database error: {}", e))
        }
    }
}

/// Check if the user has completed onboarding
#[tauri::command]
pub async fn is_onboarded(
    database: State<'_, Arc<Database>>,
) -> Result<bool, String> {
    match database.is_onboarded().await {
        Ok(is_onboarded) => Ok(is_onboarded),
        Err(e) => {
            log::error!("Failed to check onboarding status: {}", e);
            Err(format!("Database error: {}", e))
        }
    }
}

/// Mark onboarding as completed
#[tauri::command]
pub async fn set_onboarding_completed(
    database: State<'_, Arc<Database>>,
) -> Result<(), String> {
    match database.set_onboarding_completed().await {
        Ok(_) => {
            log::info!("✅ Onboarding marked as completed");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to set onboarding completed: {}", e);
            Err(format!("Database error: {}", e))
        }
    }
}

/// Get a user preference value
#[tauri::command]
pub async fn get_preference(
    key: String,
    database: State<'_, Arc<Database>>,
) -> Result<Option<String>, String> {
    match database.get_preference(&key).await {
        Ok(value) => Ok(value),
        Err(e) => {
            log::error!("Failed to get preference {}: {}", key, e);
            Err(format!("Database error: {}", e))
        }
    }
}

/// Set a user preference value
#[tauri::command]
pub async fn set_preference(
    key: String,
    value: String,
    database: State<'_, Arc<Database>>,
) -> Result<(), String> {
    match database.set_preference(&key, &value).await {
        Ok(_) => {
            log::debug!("✅ Set preference {} = {}", key, value);
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to set preference {} = {}: {}", key, value, e);
            Err(format!("Database error: {}", e))
        }
    }
}

/// Debug command to get current onboarding state
#[tauri::command]
pub async fn debug_onboarding_state(
    database: State<'_, Arc<Database>>,
) -> Result<serde_json::Value, String> {
    let first_time = database.is_first_time_install().await.unwrap_or(true);
    let onboarded = database.is_onboarded().await.unwrap_or(false);
    
    Ok(serde_json::json!({
        "is_first_time_install": first_time,
        "is_onboarded": onboarded,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
} 