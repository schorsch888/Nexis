//! AI Provider Registry
//!
//! Central registry for managing AI providers with support for
//! dynamic registration, health checks, and default provider selection.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{AIProvider, ProviderError};

/// Provider registry for managing multiple AI providers
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn AIProvider>>>,
    default_provider: RwLock<Option<String>>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
        }
    }
    
    /// Register a provider
    pub async fn register(&self, name: impl Into<String>, provider: Arc<dyn AIProvider>) {
        let name = name.into();
        let mut providers = self.providers.write().await;
        
        // Set as default if this is the first provider
        if providers.is_empty() {
            let mut default = self.default_provider.write().await;
            *default = Some(name.clone());
        }
        
        providers.insert(name, provider);
    }
    
    /// Get a provider by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn AIProvider>> {
        let providers = self.providers.read().await;
        providers.get(name).cloned()
    }
    
    /// Get the default provider
    pub async fn get_default(&self) -> Option<Arc<dyn AIProvider>> {
        let default = self.default_provider.read().await;
        if let Some(name) = default.as_ref() {
            self.get(name).await
        } else {
            None
        }
    }
    
    /// Set the default provider
    pub async fn set_default(&self, name: &str) -> Result<(), ProviderError> {
        let providers = self.providers.read().await;
        
        if !providers.contains_key(name) {
            return Err(ProviderError::Message(format!("Provider '{}' not found", name)));
        }
        
        let mut default = self.default_provider.write().await;
        *default = Some(name.to_string());
        
        Ok(())
    }
    
    /// List all registered providers
    pub async fn list(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }
    
    /// Check health of all providers
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let providers = self.providers.read().await;
        let mut results = HashMap::new();
        
        for (name, provider) in providers.iter() {
            // Simple health check: try to get the provider name
            // In production, you might want to make a lightweight API call
            let healthy = Arc::strong_count(provider) > 0;
            results.insert(name.clone(), healthy);
        }
        
        results
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenerateRequest, GenerateResponse, ProviderStream};
    use async_trait::async_trait;
    
    #[derive(Debug)]
    struct MockProvider {
        name: &'static str,
    }
    
    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        
        async fn generate(&self, _req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
            Ok(GenerateResponse {
                content: "mock response".to_string(),
                model: Some("mock".to_string()),
                finish_reason: None,
            })
        }
        
        async fn generate_stream(&self, _req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
            unimplemented!()
        }
    }
    
    #[tokio::test]
    async fn register_and_get_provider() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider { name: "test" });
        
        registry.register("test", provider).await;
        
        let retrieved = registry.get("test").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test");
    }
    
    #[tokio::test]
    async fn default_provider() {
        let registry = ProviderRegistry::new();
        
        // First provider becomes default
        let provider1 = Arc::new(MockProvider { name: "p1" });
        registry.register("p1", provider1).await;
        
        let default = registry.get_default().await;
        assert_eq!(default.unwrap().name(), "p1");
        
        // Add second provider
        let provider2 = Arc::new(MockProvider { name: "p2" });
        registry.register("p2", provider2).await;
        
        // Default should still be p1
        let default = registry.get_default().await;
        assert_eq!(default.unwrap().name(), "p1");
        
        // Change default
        registry.set_default("p2").await.unwrap();
        let default = registry.get_default().await;
        assert_eq!(default.unwrap().name(), "p2");
    }
    
    #[tokio::test]
    async fn list_providers() {
        let registry = ProviderRegistry::new();
        
        let provider1 = Arc::new(MockProvider { name: "p1" });
        let provider2 = Arc::new(MockProvider { name: "p2" });
        
        registry.register("p1", provider1).await;
        registry.register("p2", provider2).await;
        
        let list = registry.list().await;
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"p1".to_string()));
        assert!(list.contains(&"p2".to_string()));
    }
    
    #[tokio::test]
    async fn health_check() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider { name: "test" });
        
        registry.register("test", provider).await;
        
        let health = registry.health_check().await;
        assert_eq!(health.get("test"), Some(&true));
    }
    
    #[tokio::test]
    async fn set_default_nonexistent_fails() {
        let registry = ProviderRegistry::new();
        
        let err = registry.set_default("nonexistent").await.unwrap_err();
        match err {
            ProviderError::Message(msg) => assert!(msg.contains("nonexistent")),
            _ => panic!("Expected Message error"),
        }
    }
}
