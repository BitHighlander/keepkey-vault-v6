use serde::{Serialize, Deserialize};
use crate::features::DeviceFeatures;

/// Bootloader check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderCheck {
    pub needs_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub is_critical: bool,
    pub bootloader_mode: bool,
}

/// Version comparison result
#[derive(Debug, Clone, PartialEq)]
pub enum VersionComparison {
    Less,
    Equal,
    Greater,
}

/// Check bootloader status against minimum required version
pub fn check_bootloader_status(features: &DeviceFeatures) -> BootloaderCheck {
    let current_version = features.bootloader_version.clone().unwrap_or_else(|| "0.0.0".to_string());
    let latest_version = "2.1.4".to_string(); // Minimum required version
    let bootloader_mode = features.bootloader_mode.unwrap_or(false);
    
    let comparison = compare_versions(&current_version, &latest_version);
    let needs_update = comparison == VersionComparison::Less;
    let is_critical = needs_update && !bootloader_mode;
    
    BootloaderCheck {
        needs_update,
        current_version,
        latest_version,
        is_critical,
        bootloader_mode,
    }
}

/// Compare two semantic version strings
pub fn compare_versions(version1: &str, version2: &str) -> VersionComparison {
    let v1_parts: Vec<u32> = version1
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let v2_parts: Vec<u32> = version2
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    // Pad with zeros if needed
    let max_len = v1_parts.len().max(v2_parts.len());
    let mut v1_padded = v1_parts.clone();
    let mut v2_padded = v2_parts.clone();
    
    v1_padded.resize(max_len, 0);
    v2_padded.resize(max_len, 0);
    
    for (a, b) in v1_padded.iter().zip(v2_padded.iter()) {
        if a < b {
            return VersionComparison::Less;
        } else if a > b {
            return VersionComparison::Greater;
        }
    }
    
    VersionComparison::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert_eq!(compare_versions("1.0.0", "1.0.0"), VersionComparison::Equal);
        assert_eq!(compare_versions("1.0.0", "1.0.1"), VersionComparison::Less);
        assert_eq!(compare_versions("1.0.1", "1.0.0"), VersionComparison::Greater);
        assert_eq!(compare_versions("2.1.3", "2.1.4"), VersionComparison::Less);
        assert_eq!(compare_versions("2.1.4", "2.1.4"), VersionComparison::Equal);
        assert_eq!(compare_versions("2.1.5", "2.1.4"), VersionComparison::Greater);
    }

    #[test]
    fn test_bootloader_check() {
        let mut features = DeviceFeatures::default();
        features.bootloader_version = Some("2.1.3".to_string());
        features.bootloader_mode = Some(false);
        
        let check = check_bootloader_status(&features);
        assert!(check.needs_update);
        assert!(check.is_critical);
        assert_eq!(check.current_version, "2.1.3");
        assert_eq!(check.latest_version, "2.1.4");
    }

    #[test]
    fn test_bootloader_check_current() {
        let mut features = DeviceFeatures::default();
        features.bootloader_version = Some("2.1.4".to_string());
        features.bootloader_mode = Some(false);
        
        let check = check_bootloader_status(&features);
        assert!(!check.needs_update);
        assert!(!check.is_critical);
    }
} 