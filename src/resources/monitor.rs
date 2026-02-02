//! Resource monitoring service

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

use super::disk::get_disk_usage_percent;
use super::hints::{CRITICAL_HINT, WARNING_HINT};
use crate::config::ResourceSettings;

/// Resource level based on usage thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceLevel {
    /// Normal operation (below warning threshold)
    Normal,
    /// Warning level (between warning and critical thresholds)
    Warning,
    /// Critical level (above critical threshold)
    Critical,
}

impl Default for ResourceLevel {
    fn default() -> Self {
        ResourceLevel::Normal
    }
}

impl std::fmt::Display for ResourceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceLevel::Normal => write!(f, "normal"),
            ResourceLevel::Warning => write!(f, "warning"),
            ResourceLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Current resource status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    /// Current resource level
    pub level: ResourceLevel,
    /// Disk usage percentage
    pub disk_usage_percent: f32,
    /// Optional hint message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// List of active warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl Default for ResourceStatus {
    fn default() -> Self {
        ResourceStatus {
            level: ResourceLevel::Normal,
            disk_usage_percent: 0.0,
            hint: None,
            warnings: Vec::new(),
        }
    }
}

/// Hub status summary for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubStatusSummary {
    /// Current resource level
    pub level: ResourceLevel,
    /// Optional hint message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// List of active warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl From<&ResourceStatus> for HubStatusSummary {
    fn from(status: &ResourceStatus) -> Self {
        HubStatusSummary {
            level: status.level,
            hint: status.hint.clone(),
            warnings: status.warnings.clone(),
        }
    }
}

/// Resource monitor service
pub struct ResourceMonitor {
    settings: ResourceSettings,
    current_status: Arc<RwLock<ResourceStatus>>,
    monitor_path: PathBuf,
}

impl ResourceMonitor {
    /// Create a new resource monitor with the given settings
    pub fn new(settings: ResourceSettings) -> Self {
        let monitor_path = settings
            .monitor_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        ResourceMonitor {
            settings,
            current_status: Arc::new(RwLock::new(ResourceStatus::default())),
            monitor_path,
        }
    }

    /// Get the current resource status
    pub fn get_status(&self) -> ResourceStatus {
        self.current_status.read().clone()
    }

    /// Get hub status summary for API responses
    pub fn get_hub_status_summary(&self) -> Option<HubStatusSummary> {
        let status = self.current_status.read();
        if status.level == ResourceLevel::Normal {
            None
        } else {
            Some(HubStatusSummary::from(&*status))
        }
    }

    /// Check if a new agent can be accepted
    ///
    /// Returns false when at critical level
    pub fn check_can_accept_agent(&self, status: &ResourceStatus) -> bool {
        status.level != ResourceLevel::Critical
    }

    /// Check if content from an agent can be accepted
    ///
    /// At critical level, only known agents can create content
    pub fn check_can_accept_content(&self, status: &ResourceStatus, agent_known: bool) -> bool {
        match status.level {
            ResourceLevel::Normal | ResourceLevel::Warning => true,
            ResourceLevel::Critical => agent_known,
        }
    }

    /// Update the resource status by checking disk usage
    pub fn update_status(&self) {
        let disk_usage = match get_disk_usage_percent(&self.monitor_path) {
            Ok(usage) => usage,
            Err(e) => {
                warn!("Failed to get disk usage: {}", e);
                return;
            }
        };

        let level = if disk_usage >= self.settings.critical_threshold as f32 {
            ResourceLevel::Critical
        } else if disk_usage >= self.settings.warning_threshold as f32 {
            ResourceLevel::Warning
        } else {
            ResourceLevel::Normal
        };

        let hint = match level {
            ResourceLevel::Normal => None,
            ResourceLevel::Warning => Some(WARNING_HINT.to_string()),
            ResourceLevel::Critical => Some(CRITICAL_HINT.to_string()),
        };

        let mut warnings = Vec::new();
        if level == ResourceLevel::Warning {
            warnings.push(format!(
                "Disk usage at {:.1}% (warning threshold: {}%)",
                disk_usage, self.settings.warning_threshold
            ));
        } else if level == ResourceLevel::Critical {
            warnings.push(format!(
                "Disk usage at {:.1}% (critical threshold: {}%)",
                disk_usage, self.settings.critical_threshold
            ));
        }

        let new_status = ResourceStatus {
            level,
            disk_usage_percent: disk_usage,
            hint,
            warnings,
        };

        // Log level changes
        let old_level = self.current_status.read().level;
        if old_level != level {
            match level {
                ResourceLevel::Normal => info!("Resource level returned to normal"),
                ResourceLevel::Warning => warn!(
                    "Resource level changed to WARNING: disk usage at {:.1}%",
                    disk_usage
                ),
                ResourceLevel::Critical => warn!(
                    "Resource level changed to CRITICAL: disk usage at {:.1}%",
                    disk_usage
                ),
            }
        }

        *self.current_status.write() = new_status;
        debug!("Resource status updated: disk usage at {:.1}%", disk_usage);
    }

    /// Start the background monitoring task
    pub fn start_monitoring(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.settings.check_interval_sec;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;
                self.update_status();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> ResourceSettings {
        ResourceSettings {
            warning_threshold: 60,
            critical_threshold: 80,
            monitor_path: Some(".".to_string()),
            check_interval_sec: 60,
            project_url: "https://github.com/SandraK82/wisdom-hub".to_string(),
        }
    }

    #[test]
    fn test_resource_level_display() {
        assert_eq!(ResourceLevel::Normal.to_string(), "normal");
        assert_eq!(ResourceLevel::Warning.to_string(), "warning");
        assert_eq!(ResourceLevel::Critical.to_string(), "critical");
    }

    #[test]
    fn test_check_can_accept_agent() {
        let monitor = ResourceMonitor::new(test_settings());

        let normal_status = ResourceStatus {
            level: ResourceLevel::Normal,
            ..Default::default()
        };
        assert!(monitor.check_can_accept_agent(&normal_status));

        let warning_status = ResourceStatus {
            level: ResourceLevel::Warning,
            ..Default::default()
        };
        assert!(monitor.check_can_accept_agent(&warning_status));

        let critical_status = ResourceStatus {
            level: ResourceLevel::Critical,
            ..Default::default()
        };
        assert!(!monitor.check_can_accept_agent(&critical_status));
    }

    #[test]
    fn test_check_can_accept_content() {
        let monitor = ResourceMonitor::new(test_settings());

        let normal_status = ResourceStatus {
            level: ResourceLevel::Normal,
            ..Default::default()
        };
        assert!(monitor.check_can_accept_content(&normal_status, false));
        assert!(monitor.check_can_accept_content(&normal_status, true));

        let critical_status = ResourceStatus {
            level: ResourceLevel::Critical,
            ..Default::default()
        };
        assert!(!monitor.check_can_accept_content(&critical_status, false));
        assert!(monitor.check_can_accept_content(&critical_status, true));
    }
}
