use crate::Database;

/// Portfolio manager - handles portfolio data and caching
pub struct Portfolio {
    db: Database,
}

impl Portfolio {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // Portfolio methods can be added here
} 