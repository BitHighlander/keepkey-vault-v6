#[cfg(test)]
mod bootloader_tests {
    use keepkey_db::Database;
    use serde_json;
    use tokio;

    #[tokio::test]
    async fn test_device_features_storage_and_retrieval() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        let device_id = "test-device-123";
        let sample_features = create_sample_features(device_id, "7.6.0", false, true);
        let features_json = serde_json::to_string(&sample_features).unwrap();
        
        // Register device first
        db.register_device(device_id, Some("KK123456"), Some(&features_json))
            .await
            .expect("Failed to register device");
        
        // Update features
        db.update_device_features(device_id, &features_json)
            .await
            .expect("Failed to update device features");
        
        // Retrieve and verify
        let registry = db.get_device_registry().await.expect("Failed to get device registry");
        let device = registry.iter()
            .find(|d| d["device_id"] == device_id)
            .expect("Device not found in registry");
            
        assert_eq!(device["firmware_version"], "7.6.0");
        assert_eq!(device["initialized"], true);
        assert_eq!(device["bootloader_mode"], false);
    }

    #[tokio::test]
    async fn test_bootloader_mode_device_features() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        let device_id = "bootloader-device-456";
        let bootloader_features = create_sample_features(device_id, "2.1.3", true, false);
        let features_json = serde_json::to_string(&bootloader_features).unwrap();
        
        // Register device in bootloader mode
        db.register_device(device_id, Some("KK789012"), Some(&features_json))
            .await
            .expect("Failed to register device");
        
        // Retrieve and verify bootloader mode is correctly stored
        let registry = db.get_device_registry().await.expect("Failed to get device registry");
        let device = registry.iter()
            .find(|d| d["device_id"] == device_id)
            .expect("Device not found in registry");
            
        assert_eq!(device["bootloader_mode"], true);
        assert_eq!(device["initialized"], false);
        assert_eq!(device["firmware_version"], "2.1.3");
    }

    #[tokio::test]
    async fn test_features_update_overwrites_existing() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        let device_id = "update-test-789";
        
        // Initial features - older firmware, not initialized
        let initial_features = create_sample_features(device_id, "7.5.0", false, false);
        let initial_json = serde_json::to_string(&initial_features).unwrap();
        
        db.register_device(device_id, Some("KK345678"), Some(&initial_json))
            .await
            .expect("Failed to register device");
        
        // Updated features - newer firmware, initialized
        let updated_features = create_sample_features(device_id, "7.6.1", false, true);
        let updated_json = serde_json::to_string(&updated_features).unwrap();
        
        db.update_device_features(device_id, &updated_json)
            .await
            .expect("Failed to update device features");
        
        // Verify the update took effect
        let registry = db.get_device_registry().await.expect("Failed to get device registry");
        let device = registry.iter()
            .find(|d| d["device_id"] == device_id)
            .expect("Device not found in registry");
            
        assert_eq!(device["firmware_version"], "7.6.1");
        assert_eq!(device["initialized"], true);
    }

    #[tokio::test]
    async fn test_bootloader_version_scenarios() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        // Test different bootloader version scenarios
        let test_cases = vec![
            ("critical-update", "2.0.0", true, false, "Critical - major version behind"),
            ("minor-update", "2.1.0", true, false, "Minor version behind"),
            ("patch-update", "2.1.3", true, false, "Patch version behind"),
            ("current-version", "2.1.4", false, true, "Current version - no update needed"),
            ("future-version", "2.2.0", false, true, "Future version"),
        ];
        
        for (device_suffix, version, bootloader_mode, initialized, description) in test_cases {
            let device_id = format!("bootloader-test-{}", device_suffix);
            let features = create_sample_features(&device_id, version, bootloader_mode, initialized);
            let features_json = serde_json::to_string(&features).unwrap();
            
            db.register_device(&device_id, Some(&format!("KK{}", device_suffix)), Some(&features_json))
                .await
                .expect(&format!("Failed to register device {}", device_id));
            
            // Verify storage
            let registry = db.get_device_registry().await.expect("Failed to get device registry");
            let device = registry.iter()
                .find(|d| d["device_id"] == device_id)
                .expect(&format!("Device {} not found", device_id));
                
            assert_eq!(device["firmware_version"], version, "Version mismatch for {}", description);
            assert_eq!(device["bootloader_mode"], bootloader_mode, "Bootloader mode mismatch for {}", description);
            assert_eq!(device["initialized"], initialized, "Initialized state mismatch for {}", description);
        }
    }

    #[tokio::test]
    async fn test_invalid_features_json_handling() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        let device_id = "invalid-json-test";
        
        // Register device first
        db.register_device(device_id, Some("KK999999"), None)
            .await
            .expect("Failed to register device");
        
        // Try to update with invalid JSON
        let result = db.update_device_features(device_id, "invalid-json{").await;
        assert!(result.is_err(), "Should fail with invalid JSON");
    }

    #[tokio::test]
    async fn test_nonexistent_device_update() {
        let db = Database::new_in_memory().await.expect("Failed to create in-memory database");
        
        let features = create_sample_features("nonexistent", "7.6.0", false, true);
        let features_json = serde_json::to_string(&features).unwrap();
        
        // Try to update features for device that doesn't exist
        let result = db.update_device_features("nonexistent", &features_json).await;
        assert!(result.is_err(), "Should fail for nonexistent device");
    }

    /// Helper function to create sample device features for testing
    fn create_sample_features(
        device_id: &str, 
        version: &str, 
        bootloader_mode: bool, 
        initialized: bool
    ) -> serde_json::Value {
        serde_json::json!({
            "vendor": "KeepKey",
            "model": "KeepKey",
            "label": format!("Test Device {}", device_id),
            "firmwareVariant": None::<String>,
            "deviceId": device_id,
            "language": "english",
            "bootloaderMode": bootloader_mode,
            "version": version,
            "firmwareHash": "abc123def456",
            "bootloaderHash": "789xyz012",
            "bootloaderVersion": "2.1.4",
            "initialized": initialized,
            "imported": false,
            "noBackup": false,
            "pinProtection": true,
            "pinCached": false,
            "passphraseProtection": false,
            "passphraseCached": false,
            "wipeCodeProtection": false,
            "autoLockDelayMs": 600000,
            "policies": []
        })
    }

    /// Sample test data for different bootloader scenarios
    pub fn create_bootloader_test_scenarios() -> Vec<serde_json::Value> {
        vec![
            // Critical update needed - major version behind
            serde_json::json!({
                "device_id": "critical-bootloader-001",
                "scenario": "critical_update",
                "features": create_sample_features("critical-bootloader-001", "1.9.0", true, false),
                "expected_update": true,
                "expected_severity": "critical",
                "description": "Major version behind - critical security update required"
            }),
            
            // High priority update - minor version behind
            serde_json::json!({
                "device_id": "high-bootloader-002", 
                "scenario": "high_update",
                "features": create_sample_features("high-bootloader-002", "2.0.5", true, false),
                "expected_update": true,
                "expected_severity": "high",
                "description": "Minor version behind - important features missing"
            }),
            
            // Medium priority update - patch version behind
            serde_json::json!({
                "device_id": "medium-bootloader-003",
                "scenario": "medium_update", 
                "features": create_sample_features("medium-bootloader-003", "2.1.3", true, false),
                "expected_update": true,
                "expected_severity": "medium",
                "description": "Patch version behind - recommended bug fixes available"
            }),
            
            // Current version - no update needed
            serde_json::json!({
                "device_id": "current-bootloader-004",
                "scenario": "current_version",
                "features": create_sample_features("current-bootloader-004", "2.1.4", false, true),
                "expected_update": false,
                "expected_severity": "low",
                "description": "Bootloader is current - no update required"
            }),
            
            // Device in normal mode with current firmware
            serde_json::json!({
                "device_id": "normal-mode-005",
                "scenario": "normal_mode_current",
                "features": create_sample_features("normal-mode-005", "7.6.0", false, true),
                "expected_update": false,
                "expected_severity": "low", 
                "description": "Device in normal mode with current firmware"
            }),
            
            // Device in normal mode with old firmware (bootloader check should still work)
            serde_json::json!({
                "device_id": "normal-mode-old-006",
                "scenario": "normal_mode_old_firmware",
                "features": create_sample_features("normal-mode-old-006", "7.5.0", false, true),
                "expected_update": false,
                "expected_severity": "medium",
                "description": "Device in normal mode but firmware is outdated"
            })
        ]
    }
}

/// Public interface for test data generation
pub use bootloader_tests::create_bootloader_test_scenarios; 