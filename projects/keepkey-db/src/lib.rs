pub mod database;
pub mod device_registry;
pub mod portfolio;
pub mod assets;
pub mod cache;
pub mod migrations;
pub mod types;
pub mod errors;

// Re-export main types and the database
pub use database::Database;
pub use device_registry::DeviceRegistry;
pub use types::*;
pub use errors::DatabaseError;

use std::path::PathBuf;


/// Initialize the database and return a Database instance
pub async fn init_database() -> anyhow::Result<Database> {
    Database::new().await.map_err(Into::into)
}

/// Get the default database path
pub fn get_database_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".keepkey")
        .join("keepkey.db")
}

/// Check if the database file exists
pub fn database_exists() -> bool {
    get_database_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_initialization() {
        let _ = env_logger::try_init();
        let _temp_dir = TempDir::new().unwrap();
        let db = init_database().await.unwrap();
        assert!(db.health_check().await.is_ok());
    }
} 