//! Operational tooling: health checks and alerting

use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;

/// Health check status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: i64,
}

impl HealthCheck {
    /// Create a new health check
    pub fn new(name: String, status: HealthStatus, message: String, duration_ms: u64) -> Self {
        HealthCheck {
            name,
            status,
            message,
            duration_ms,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatusResponse {
    pub status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Health checker
pub struct HealthChecker {
    checks: HashMap<String, Box<dyn Fn() -> HealthCheck + Send + Sync>>,
    start_time: Instant,
    version: String,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(version: String) -> Self {
        HealthChecker {
            checks: HashMap::new(),
            start_time: Instant::now(),
            version,
        }
    }
    
    /// Register a health check
    pub fn register_check<F>(&mut self, name: String, check: F)
    where
        F: Fn() -> HealthCheck + Send + Sync + 'static,
    {
        self.checks.insert(name, Box::new(check));
    }
    
    /// Run all health checks
    pub fn check_all(&self) -> HealthStatusResponse {
        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;
        
        for check_fn in self.checks.values() {
            let check = check_fn();
            match check.status {
                HealthStatus::Unhealthy => {
                    overall_status = HealthStatus::Unhealthy;
                }
                HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                    overall_status = HealthStatus::Degraded;
                }
                _ => {}
            }
            checks.push(check);
        }
        
        HealthStatusResponse {
            status: overall_status,
            checks,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            version: self.version.clone(),
        }
    }
    
    /// Run a specific health check
    pub fn check(&self, name: &str) -> Option<HealthCheck> {
        self.checks.get(name).map(|check_fn| check_fn())
    }
}

/// Alert severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub source: String,
    pub timestamp: i64,
    pub resolved: bool,
}

impl Alert {
    /// Create a new alert
    pub fn new(
        severity: AlertSeverity,
        title: String,
        message: String,
        source: String,
    ) -> Self {
        Alert {
            id: uuid::Uuid::new_v4().to_string(),
            severity,
            title,
            message,
            source,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            resolved: false,
        }
    }
    
    /// Mark as resolved
    pub fn resolve(&mut self) {
        self.resolved = true;
    }
}

/// Alert manager
pub struct AlertManager {
    alerts: Vec<Alert>,
    max_alerts: usize,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(max_alerts: usize) -> Self {
        AlertManager {
            alerts: Vec::new(),
            max_alerts,
        }
    }
    
    /// Create an alert
    pub fn create_alert(
        &mut self,
        severity: AlertSeverity,
        title: String,
        message: String,
        source: String,
    ) -> Alert {
        let alert = Alert::new(severity, title, message, source);
        self.alerts.push(alert.clone());
        
        // Trim if too many alerts
        if self.alerts.len() > self.max_alerts {
            self.alerts.remove(0);
        }
        
        alert
    }
    
    /// Get all alerts
    pub fn get_alerts(&self) -> &[Alert] {
        &self.alerts
    }
    
    /// Get unresolved alerts
    pub fn get_unresolved(&self) -> Vec<&Alert> {
        self.alerts.iter().filter(|a| !a.resolved).collect()
    }
    
    /// Get alerts by severity
    pub fn get_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == severity)
            .collect()
    }
    
    /// Resolve an alert
    pub fn resolve_alert(&mut self, id: &str) -> bool {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == id) {
            alert.resolve();
            true
        } else {
            false
        }
    }
    
    /// Clear all alerts
    pub fn clear(&mut self) {
        self.alerts.clear();
    }
}

/// System metrics for health checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_connections: u64,
}

impl SystemMetrics {
    /// Get current system metrics
    pub fn current() -> Self {
        // In a real implementation, this would use sysinfo or similar
        // For now, return placeholder values
        SystemMetrics {
            memory_usage_mb: 512,
            cpu_usage_percent: 45.0,
            disk_usage_percent: 30.0,
            network_connections: 10,
        }
    }
    
    /// Check if metrics are healthy
    pub fn is_healthy(&self) -> bool {
        self.memory_usage_mb < 4096
            && self.cpu_usage_percent < 90.0
            && self.disk_usage_percent < 90.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_check() {
        let check = HealthCheck::new(
            "test".to_string(),
            HealthStatus::Healthy,
            "OK".to_string(),
            10,
        );
        
        assert_eq!(check.status, HealthStatus::Healthy);
        assert_eq!(check.name, "test");
    }
    
    #[test]
    fn test_health_checker() {
        let mut checker = HealthChecker::new("1.0.0".to_string());
        
        checker.register_check("test".to_string(), || {
            HealthCheck::new(
                "test".to_string(),
                HealthStatus::Healthy,
                "OK".to_string(),
                5,
            )
        });
        
        let status = checker.check_all();
        assert_eq!(status.status, HealthStatus::Healthy);
        assert_eq!(status.checks.len(), 1);
    }
    
    #[test]
    fn test_alert_creation() {
        let mut manager = AlertManager::new(100);
        
        let alert = manager.create_alert(
            AlertSeverity::Warning,
            "Test Alert".to_string(),
            "This is a test".to_string(),
            "test".to_string(),
        );
        
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.resolved);
    }
    
    #[test]
    fn test_alert_resolution() {
        let mut manager = AlertManager::new(100);
        
        let alert = manager.create_alert(
            AlertSeverity::Error,
            "Test Alert".to_string(),
            "This is a test".to_string(),
            "test".to_string(),
        );
        
        let id = alert.id.clone();
        assert!(manager.resolve_alert(&id));
        
        let alerts = manager.get_alerts();
        assert!(alerts[0].resolved);
    }
    
    #[test]
    fn test_system_metrics() {
        let metrics = SystemMetrics::current();
        assert!(metrics.is_healthy());
    }
}
