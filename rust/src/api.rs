//! API stability guarantees with versioning and deprecation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ApiVersion {
    /// Create a new API version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        ApiVersion {
            major,
            minor,
            patch,
        }
    }
    
    /// Parse from string (e.g., "1.0.0")
    pub fn from_str(s: &str) -> Result<Self, ApiError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(ApiError::InvalidVersionFormat);
        }
        
        let major = parts[0].parse().map_err(|_| ApiError::InvalidVersionFormat)?;
        let minor = parts[1].parse().map_err(|_| ApiError::InvalidVersionFormat)?;
        let patch = parts[2].parse().map_err(|_| ApiError::InvalidVersionFormat)?;
        
        Ok(ApiVersion::new(major, minor, patch))
    }
    
    /// Convert to string
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
    
    /// Check if this version is compatible with another
    /// Major version must match, minor can be >=, patch can be anything
    pub fn is_compatible(&self, other: &ApiVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// API endpoint with versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub name: String,
    pub version: ApiVersion,
    pub deprecated: bool,
    pub deprecation_date: Option<i64>,
    pub removal_date: Option<i64>,
    pub description: String,
}

impl ApiEndpoint {
    /// Create a new API endpoint
    pub fn new(name: String, version: ApiVersion, description: String) -> Self {
        ApiEndpoint {
            name,
            version,
            deprecated: false,
            deprecation_date: None,
            removal_date: None,
            description,
        }
    }
    
    /// Mark as deprecated
    pub fn deprecate(&mut self, deprecation_date: i64, removal_date: i64) {
        self.deprecated = true;
        self.deprecation_date = Some(deprecation_date);
        self.removal_date = Some(removal_date);
    }
}

/// API registry for managing endpoints
pub struct ApiRegistry {
    current_version: ApiVersion,
    endpoints: HashMap<String, ApiEndpoint>,
}

impl ApiRegistry {
    /// Create a new API registry
    pub fn new(current_version: ApiVersion) -> Self {
        ApiRegistry {
            current_version,
            endpoints: HashMap::new(),
        }
    }
    
    /// Register an endpoint
    pub fn register_endpoint(&mut self, endpoint: ApiEndpoint) {
        self.endpoints.insert(endpoint.name.clone(), endpoint);
    }
    
    /// Get an endpoint
    pub fn get_endpoint(&self, name: &str) -> Option<&ApiEndpoint> {
        self.endpoints.get(name)
    }
    
    /// Check if an endpoint is deprecated
    pub fn is_deprecated(&self, name: &str) -> bool {
        self.endpoints
            .get(name)
            .map(|e| e.deprecated)
            .unwrap_or(false)
    }
    
    /// Get all deprecated endpoints
    pub fn deprecated_endpoints(&self) -> Vec<&ApiEndpoint> {
        self.endpoints
            .values()
            .filter(|e| e.deprecated)
            .collect()
    }
    
    /// Get current API version
    pub fn current_version(&self) -> ApiVersion {
        self.current_version
    }
    
    /// Update API version
    pub fn update_version(&mut self, new_version: ApiVersion) {
        self.current_version = new_version;
    }
}

/// API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    InvalidVersionFormat,
    EndpointNotFound(String),
    VersionMismatch(String),
    DeprecatedEndpoint(String),
    RemovedEndpoint(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::InvalidVersionFormat => write!(f, "Invalid version format"),
            ApiError::EndpointNotFound(name) => write!(f, "Endpoint not found: {}", name),
            ApiError::VersionMismatch(msg) => write!(f, "Version mismatch: {}", msg),
            ApiError::DeprecatedEndpoint(name) => write!(f, "Endpoint deprecated: {}", name),
            ApiError::RemovedEndpoint(name) => write!(f, "Endpoint removed: {}", name),
        }
    }
}

impl std::error::Error for ApiError {}

/// API request with version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub endpoint: String,
    pub version: Option<ApiVersion>,
    pub data: serde_json::Value,
}

/// API response with version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub version: ApiVersion,
    pub data: serde_json::Value,
    pub deprecated: bool,
    pub deprecation_notice: Option<String>,
}

/// API handler with versioning
pub struct ApiHandler {
    registry: ApiRegistry,
}

impl ApiHandler {
    /// Create a new API handler
    pub fn new(registry: ApiRegistry) -> Self {
        ApiHandler { registry }
    }
    
    /// Handle an API request
    pub fn handle_request(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Check if endpoint exists
        let endpoint = self
            .registry
            .get_endpoint(&request.endpoint)
            .ok_or_else(|| ApiError::EndpointNotFound(request.endpoint.clone()))?;
        
        // Check if endpoint is removed
        if let Some(removal_date) = endpoint.removal_date {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            if now > removal_date {
                return Err(ApiError::RemovedEndpoint(request.endpoint));
            }
        }
        
        // Check version compatibility
        if let Some(request_version) = request.version {
            if !request_version.is_compatible(&endpoint.version) {
                return Err(ApiError::VersionMismatch(format!(
                    "Request version {} incompatible with endpoint version {}",
                    request_version, endpoint.version
                )));
            }
        }
        
        // Check deprecation
        let deprecation_notice = if endpoint.deprecated {
            Some(format!(
                "Endpoint deprecated. Will be removed on {}",
                endpoint
                    .removal_date
                    .map(|d| {
                        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(d, 0);
                        datetime.map(|dt| dt.to_rfc3339()).unwrap_or_default()
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        } else {
            None
        };
        
        // Return response
        Ok(ApiResponse {
            version: self.registry.current_version(),
            data: serde_json::json!({"status": "success"}),
            deprecated: endpoint.deprecated,
            deprecation_notice,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_version() {
        let v1 = ApiVersion::new(1, 0, 0);
        let v2 = ApiVersion::new(1, 1, 0);
        let v3 = ApiVersion::new(2, 0, 0);
        
        assert!(v2.is_compatible(&v1));
        assert!(!v1.is_compatible(&v3));
        assert_eq!(v1.to_string(), "1.0.0");
    }
    
    #[test]
    fn test_api_version_parse() {
        let v = ApiVersion::from_str("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }
    
    #[test]
    fn test_endpoint_deprecation() {
        let mut endpoint = ApiEndpoint::new(
            "test".to_string(),
            ApiVersion::new(1, 0, 0),
            "Test endpoint".to_string(),
        );
        
        assert!(!endpoint.deprecated);
        
        endpoint.deprecate(1000, 2000);
        assert!(endpoint.deprecated);
        assert_eq!(endpoint.deprecation_date, Some(1000));
    }
    
    #[test]
    fn test_api_registry() {
        let mut registry = ApiRegistry::new(ApiVersion::new(1, 0, 0));
        
        let endpoint = ApiEndpoint::new(
            "test".to_string(),
            ApiVersion::new(1, 0, 0),
            "Test endpoint".to_string(),
        );
        
        registry.register_endpoint(endpoint);
        
        assert!(registry.get_endpoint("test").is_some());
        assert!(!registry.is_deprecated("test"));
    }
    
    #[test]
    fn test_api_handler() {
        let mut registry = ApiRegistry::new(ApiVersion::new(1, 0, 0));
        
        let endpoint = ApiEndpoint::new(
            "test".to_string(),
            ApiVersion::new(1, 0, 0),
            "Test endpoint".to_string(),
        );
        
        registry.register_endpoint(endpoint);
        
        let handler = ApiHandler::new(registry);
        
        let request = ApiRequest {
            endpoint: "test".to_string(),
            version: Some(ApiVersion::new(1, 0, 0)),
            data: serde_json::json!({}),
        };
        
        let response = handler.handle_request(request);
        assert!(response.is_ok());
    }
}
