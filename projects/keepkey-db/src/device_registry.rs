use crate::Database;

/// Device Registry manager - handles device setup flow and tracking
pub struct DeviceRegistry {
    db: Database,
}

impl DeviceRegistry {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // Device registry methods are implemented directly in Database
    // This module can be extended for more complex device registry logic
} 