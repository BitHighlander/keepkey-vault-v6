use crate::errors::Result;
use crate::migrations::apply_migrations;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Main database manager
pub struct Database {
    connection: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Create a new database instance
    pub async fn new() -> Result<Self> {
        let path = crate::get_database_path();
        let db = Self::open_at_path(path).await?;
        Ok(db)
    }

    /// Create a database instance at a specific path
    pub async fn open_at_path(path: PathBuf) -> Result<Self> {
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        log::info!("Opening database at: {:?}", path);

        // Open connection with proper flags
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        // Apply migrations
        if let Err(e) = apply_migrations(&conn) {
            log::error!("Failed to apply migrations: {}", e);
            return Err(e);
        }

        let db = Database {
            connection: Arc::new(Mutex::new(conn)),
            path,
        };

        log::info!("Database initialized successfully");
        Ok(db)
    }

    /// Create an in-memory database instance for testing
    pub async fn new_in_memory() -> Result<Self> {
        log::info!("Creating in-memory database for testing");

        // Create in-memory connection
        let conn = Connection::open_in_memory()?;

        // Apply migrations
        if let Err(e) = apply_migrations(&conn) {
            log::error!("Failed to apply migrations to in-memory database: {}", e);
            return Err(e);
        }

        let db = Database {
            connection: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        };

        log::info!("In-memory database initialized successfully");
        Ok(db)
    }

    /// Get the database path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Health check - ensure database is accessible
    pub async fn health_check(&self) -> Result<()> {
        let conn = self.connection.lock().await;
        match conn.query_row("SELECT 1", [], |_| Ok(())) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("Health check failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Execute a closure with database connection
    pub async fn with_connection<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R> + Send,
        R: Send,
    {
        let conn = self.connection.lock().await;
        f(&*conn)
    }

    /// Execute a transaction
    pub async fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R> + Send,
        R: Send,
    {
        let mut conn = self.connection.lock().await;
        let tx = conn.transaction()?;
        
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    /// Get current UNIX timestamp
    pub fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    // ========== Device Registry Methods ==========

    /// Register a device in the database
    pub async fn register_device(
        &self,
        device_id: &str,
        serial_number: Option<&str>,
        features: Option<&str>,
    ) -> Result<()> {
        let now = Self::current_timestamp();
        
        self.with_connection(|conn| {
            // Parse features if provided
            let (vendor, model, label, firmware_variant, firmware_version, 
                 bootloader_mode, initialized, pin_protection, passphrase_protection) = 
                if let Some(features_json) = features {
                    if let Ok(features) = serde_json::from_str::<serde_json::Value>(features_json) {
                        (
                            features.get("vendor").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            features.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            features.get("label").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            features.get("firmwareVariant").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            features.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            features.get("bootloaderMode").and_then(|v| v.as_bool()).unwrap_or(false),
                            features.get("initialized").and_then(|v| v.as_bool()).unwrap_or(false),
                            features.get("pinProtection").and_then(|v| v.as_bool()).unwrap_or(false),
                            features.get("passphraseProtection").and_then(|v| v.as_bool()).unwrap_or(false),
                        )
                    } else {
                        (None, None, None, None, None, false, false, false, false)
                    }
                } else {
                    (None, None, None, None, None, false, false, false, false)
                };
            
            conn.execute(
                "INSERT OR REPLACE INTO devices (
                    device_id, first_seen, last_seen, features, serial_number,
                    vendor, model, label, firmware_variant, firmware_version,
                    bootloader_mode, initialized, pin_protection, passphrase_protection,
                    setup_complete, setup_step_completed
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    device_id, now, now, features, serial_number,
                    vendor, model, label, firmware_variant, firmware_version,
                    bootloader_mode, initialized, pin_protection, passphrase_protection,
                    false, 0
                ],
            )?;
            
            log::info!("Registered device: {}", device_id);
            Ok(())
        }).await
    }

    /// Check if a device needs setup
    pub async fn device_needs_setup(&self, device_id: &str) -> Result<bool> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT setup_complete FROM devices WHERE device_id = ?1"
            )?;
            
            let setup_complete: bool = stmt.query_row([device_id], |row| {
                Ok(row.get(0).unwrap_or(false))
            })?;
            
            Ok(!setup_complete)
        }).await
    }

    /// Update device setup step
    pub async fn update_device_setup_step(&self, device_id: &str, step: u8) -> Result<()> {
        let now = Self::current_timestamp();
        
        self.with_connection(|conn| {
            let updated = conn.execute(
                "UPDATE devices SET setup_step_completed = ?1, setup_started_at = COALESCE(setup_started_at, ?2), last_seen = ?3 
                 WHERE device_id = ?4",
                rusqlite::params![step, now, now, device_id],
            )?;
            
            if updated == 0 {
                return Err(crate::errors::DatabaseError::DeviceNotFound(device_id.to_string()));
            }
            
            log::info!("Updated setup step for device {}: step {}", device_id, step);
            Ok(())
        }).await
    }

    /// Mark device setup as complete
    pub async fn mark_device_setup_complete(
        &self,
        device_id: &str,
        eth_address: Option<&str>,
    ) -> Result<()> {
        let now = Self::current_timestamp();
        
        self.with_connection(|conn| {
            let updated = conn.execute(
                "UPDATE devices SET 
                    setup_complete = TRUE, 
                    setup_step_completed = 4,
                    setup_completed_at = ?1,
                    eth_address = ?2,
                    last_seen = ?3
                 WHERE device_id = ?4",
                rusqlite::params![now, eth_address, now, device_id],
            )?;
            
            if updated == 0 {
                return Err(crate::errors::DatabaseError::DeviceNotFound(device_id.to_string()));
            }
            
            log::info!("Marked device setup as complete: {}", device_id);
            Ok(())
        }).await
    }

    /// Reset device setup (for testing/debugging)
    pub async fn reset_device_setup(&self, device_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            let updated = conn.execute(
                "UPDATE devices SET 
                    setup_complete = FALSE,
                    setup_step_completed = 0,
                    setup_started_at = NULL,
                    setup_completed_at = NULL,
                    eth_address = NULL
                 WHERE device_id = ?1",
                [device_id],
            )?;
            
            if updated == 0 {
                return Err(crate::errors::DatabaseError::DeviceNotFound(device_id.to_string()));
            }
            
            log::info!("Reset device setup: {}", device_id);
            Ok(())
        }).await
    }

    /// Get devices with incomplete setup
    pub async fn get_incomplete_setup_devices(&self) -> Result<Vec<serde_json::Value>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT device_id, serial_number, setup_step_completed, features 
                 FROM devices 
                 WHERE setup_complete = FALSE 
                 ORDER BY first_seen DESC"
            )?;
            
            let devices = stmt.query_map([], |row| {
                let device_id: String = row.get(0)?;
                let serial_number: Option<String> = row.get(1)?;
                let setup_step: i64 = row.get(2)?;
                let features: Option<String> = row.get(3)?;
                
                Ok(serde_json::json!({
                    "device_id": device_id,
                    "serial_number": serial_number,
                    "setup_step_completed": setup_step,
                    "features": features
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
            
            Ok(devices)
        }).await
    }

    /// Update device features in the database
    pub async fn update_device_features(&self, device_id: &str, features_json: &str) -> Result<()> {
        let now = Self::current_timestamp();
        
        self.with_connection(|conn| {
            // Parse features to extract key fields for indexed columns
            if let Ok(features) = serde_json::from_str::<serde_json::Value>(features_json) {
                let vendor = features.get("vendor").and_then(|v| v.as_str());
                let model = features.get("model").and_then(|v| v.as_str());
                let label = features.get("label").and_then(|v| v.as_str());
                let firmware_variant = features.get("firmwareVariant").and_then(|v| v.as_str());
                let firmware_version = features.get("version").and_then(|v| v.as_str());
                let bootloader_mode = features.get("bootloaderMode").and_then(|v| v.as_bool()).unwrap_or(false);
                let initialized = features.get("initialized").and_then(|v| v.as_bool()).unwrap_or(false);
                let pin_protection = features.get("pinProtection").and_then(|v| v.as_bool()).unwrap_or(false);
                let passphrase_protection = features.get("passphraseProtection").and_then(|v| v.as_bool()).unwrap_or(false);
                
                let updated = conn.execute(
                    "UPDATE devices SET 
                        vendor = ?1, model = ?2, label = ?3, firmware_variant = ?4, firmware_version = ?5,
                        bootloader_mode = ?6, initialized = ?7, pin_protection = ?8, passphrase_protection = ?9,
                        features = ?10, last_seen = ?11
                     WHERE device_id = ?12",
                    rusqlite::params![
                        vendor, model, label, firmware_variant, firmware_version,
                        bootloader_mode, initialized, pin_protection, passphrase_protection,
                        features_json, now, device_id
                    ],
                )?;
                
                if updated == 0 {
                    return Err(crate::errors::DatabaseError::DeviceNotFound(device_id.to_string()));
                }
                
                log::info!("Updated device features for device: {}", device_id);
                Ok(())
            } else {
                Err(crate::errors::DatabaseError::InvalidData("Invalid features JSON".to_string()))
            }
        }).await
    }

    /// Get device registry (all devices)
    pub async fn get_device_registry(&self) -> Result<Vec<serde_json::Value>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT device_id, vendor, model, label, firmware_variant, firmware_version,
                        bootloader_mode, initialized, pin_protection, passphrase_protection,
                        first_seen, last_seen, features, serial_number, setup_complete,
                        setup_step_completed, eth_address, setup_started_at, setup_completed_at
                 FROM devices 
                 ORDER BY last_seen DESC"
            )?;
            
            let devices = stmt.query_map([], |row| {
                Ok(serde_json::json!({
                    "device_id": row.get::<_, String>(0)?,
                    "vendor": row.get::<_, Option<String>>(1)?,
                    "model": row.get::<_, Option<String>>(2)?,
                    "label": row.get::<_, Option<String>>(3)?,
                    "firmware_variant": row.get::<_, Option<String>>(4)?,
                    "firmware_version": row.get::<_, Option<String>>(5)?,
                    "bootloader_mode": row.get::<_, bool>(6)?,
                    "initialized": row.get::<_, bool>(7)?,
                    "pin_protection": row.get::<_, bool>(8)?,
                    "passphrase_protection": row.get::<_, bool>(9)?,
                    "first_seen": row.get::<_, i64>(10)?,
                    "last_seen": row.get::<_, i64>(11)?,
                    "features": row.get::<_, Option<String>>(12)?,
                    "serial_number": row.get::<_, Option<String>>(13)?,
                    "setup_complete": row.get::<_, bool>(14)?,
                    "setup_step_completed": row.get::<_, i64>(15)?,
                    "eth_address": row.get::<_, Option<String>>(16)?,
                    "setup_started_at": row.get::<_, Option<i64>>(17)?,
                    "setup_completed_at": row.get::<_, Option<i64>>(18)?
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
            
            Ok(devices)
        }).await
    }

    /// Get a specific device by ID
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<serde_json::Value>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT device_id, vendor, model, label, firmware_variant, firmware_version,
                        bootloader_mode, initialized, pin_protection, passphrase_protection,
                        first_seen, last_seen, features, serial_number, setup_complete,
                        setup_step_completed, eth_address, setup_started_at, setup_completed_at
                 FROM devices 
                 WHERE device_id = ?1"
            )?;
            
            let device = stmt.query_row([device_id], |row| {
                Ok(serde_json::json!({
                    "device_id": row.get::<_, String>(0)?,
                    "vendor": row.get::<_, Option<String>>(1)?,
                    "model": row.get::<_, Option<String>>(2)?,
                    "label": row.get::<_, Option<String>>(3)?,
                    "firmware_variant": row.get::<_, Option<String>>(4)?,
                    "firmware_version": row.get::<_, Option<String>>(5)?,
                    "bootloader_mode": row.get::<_, bool>(6)?,
                    "initialized": row.get::<_, bool>(7)?,
                    "pin_protection": row.get::<_, bool>(8)?,
                    "passphrase_protection": row.get::<_, bool>(9)?,
                    "first_seen": row.get::<_, i64>(10)?,
                    "last_seen": row.get::<_, i64>(11)?,
                    "features": row.get::<_, Option<String>>(12)?,
                    "serial_number": row.get::<_, Option<String>>(13)?,
                    "setup_complete": row.get::<_, bool>(14)?,
                    "setup_step_completed": row.get::<_, i64>(15)?,
                    "eth_address": row.get::<_, Option<String>>(16)?,
                    "setup_started_at": row.get::<_, Option<i64>>(17)?,
                    "setup_completed_at": row.get::<_, Option<i64>>(18)?
                }))
            }).optional()?;
            
            Ok(device)
        }).await
    }

    /// Get ETH address for a device
    pub async fn get_device_eth_address(&self, device_id: &str) -> Result<Option<String>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT eth_address FROM devices WHERE device_id = ?1")?;
            let address = stmt.query_row([device_id], |row| {
                Ok(row.get::<_, Option<String>>(0)?)
            }).optional()?;
            
            Ok(address.flatten())
        }).await
    }

    // ========== Onboarding/Preferences Methods ==========

    /// Check if user has completed onboarding
    pub async fn is_onboarded(&self) -> Result<bool> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT val FROM meta WHERE key = 'onboarding_completed'")?;
            let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
            Ok(result.map(|v| v == "true").unwrap_or(false))
        }).await
    }

    /// Mark onboarding as completed
    pub async fn set_onboarding_completed(&self) -> Result<()> {
        let timestamp = Self::current_timestamp();
        
        self.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, val) VALUES ('onboarding_completed', 'true')",
                [],
            )?;
            
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, val) VALUES ('onboarding_timestamp', ?1)",
                [timestamp.to_string()],
            )?;
            
            log::info!("Onboarding marked as completed");
            Ok(())
        }).await
    }

    /// Set user preference
    pub async fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        let pref_key = format!("pref_{}", key);
        
        self.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, val) VALUES (?1, ?2)",
                rusqlite::params![pref_key, value],
            )?;
            Ok(())
        }).await
    }

    /// Get user preference
    pub async fn get_preference(&self, key: &str) -> Result<Option<String>> {
        let pref_key = format!("pref_{}", key);
        
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT val FROM meta WHERE key = ?1")?;
            let result: Option<String> = stmt.query_row([pref_key], |row| row.get(0)).ok();
            Ok(result)
        }).await
    }

    /// Check if this is a first-time install
    pub async fn is_first_time_install(&self) -> Result<bool> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT val FROM meta WHERE key = 'first_install_timestamp'")?;
            let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
            
            // If no timestamp exists, it's a first install
            Ok(result.is_none())
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_creation() {
        let _ = env_logger::try_init();
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let db = Database::open_at_path(db_path).await.unwrap();
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_device_registration() {
        let _ = env_logger::try_init();
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open_at_path(db_path).await.unwrap();

        // Register a device
        db.register_device("test_device", Some("12345"), Some("{}")).await.unwrap();
        
        // Check if device needs setup
        assert!(db.device_needs_setup("test_device").await.unwrap());
        
        // Complete setup
        db.mark_device_setup_complete("test_device", Some("0x1234")).await.unwrap();
        
        // Should no longer need setup
        assert!(!db.device_needs_setup("test_device").await.unwrap());
        
        // Check ETH address
        let eth_addr = db.get_device_eth_address("test_device").await.unwrap();
        assert_eq!(eth_addr, Some("0x1234".to_string()));
    }
} 