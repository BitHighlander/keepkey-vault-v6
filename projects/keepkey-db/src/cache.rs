use crate::Database;

/// Cache manager - handles frontloading and cached data
pub struct Cache {
    db: Database,
}

impl Cache {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // Cache methods can be added here
} 