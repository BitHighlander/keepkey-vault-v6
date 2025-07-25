// commands/events.rs - Event handling utilities

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub struct FrontendReadyState {
    pub is_ready: bool,
    pub queued_events: Vec<QueuedEvent>,
}

impl Default for FrontendReadyState {
    fn default() -> Self {
        Self {
            is_ready: false,
            queued_events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub event_name: String,
    pub payload: serde_json::Value,
    pub timestamp: u64,
}

// Add frontend readiness state and queued events
lazy_static::lazy_static! {
    static ref FRONTEND_READY_STATE: Arc<RwLock<FrontendReadyState>> = Arc::new(RwLock::new(FrontendReadyState::default()));
    // One-time initialization flag to prevent duplicate ready signals
    static ref FRONTEND_READY_ONCE: Arc<tokio::sync::Mutex<bool>> = Arc::new(tokio::sync::Mutex::new(false));
}

/// Signal that the frontend is ready to receive events
#[tauri::command]
pub async fn frontend_ready(app: AppHandle) -> Result<(), String> {
    log::info!("🎯 Frontend ready signal received - enabling event emission");
    
    // Check if we've already processed frontend ready to avoid duplicates
    let mut ready_once = FRONTEND_READY_ONCE.lock().await;
    if *ready_once {
        log::warn!("⚠️ Frontend ready signal already processed - ignoring duplicate");
        return Ok(());
    }
    *ready_once = true;
    drop(ready_once);

    // Mark frontend as ready and process queued events
    let mut state = FRONTEND_READY_STATE.write().await;
    state.is_ready = true;
    
    if !state.queued_events.is_empty() {
        log::info!("📦 Flushing {} queued events to frontend", state.queued_events.len());
        
        // Process all queued events
        for event in state.queued_events.drain(..) {
            if let Err(e) = app.emit(&event.event_name, &event.payload) {
                log::error!("❌ Failed to emit queued event {}: {}", event.event_name, e);
            } else {
                log::debug!("📡 Emitted queued event: {}", event.event_name);
            }
        }
        
        log::info!("✅ All queued events have been sent to frontend");
    }
    
    Ok(())
}

/// Emit an event to frontend or queue it if frontend isn't ready
pub async fn emit_or_queue_event(
    app: &AppHandle,
    event_name: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let state = FRONTEND_READY_STATE.read().await;
    
    if state.is_ready {
        // Frontend is ready - emit immediately
        if let Err(e) = app.emit(event_name, &payload) {
            log::error!("❌ Failed to emit event {}: {}", event_name, e);
            return Err(format!("Failed to emit event: {}", e));
        }
        log::debug!("📡 Emitted event: {}", event_name);
    } else {
        // Frontend not ready - queue the event
        drop(state); // Release read lock
        let mut state = FRONTEND_READY_STATE.write().await;
        
        let queued_event = QueuedEvent {
            event_name: event_name.to_string(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        state.queued_events.push(queued_event);
        let queue_size = state.queued_events.len();
        
        println!("📋 Queued event: {} (total queued: {})", event_name, queue_size);
    }
    
    Ok(())
} 