//! Tool calling system for AI agents
//!
//! This module provides a standardized way for AI agents to call tools
//! and execute actions in the real world.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Tool execution error
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    
    #[error("invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("timeout after {0}ms")]
    Timeout(u64),
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g., "web_search")
    pub name: String,
    
    /// Human-readable description
    pub description: String,
    
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

/// Tool call request from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique call ID
    pub id: String,
    
    /// Tool name
    pub name: String,
    
    /// Parameters as JSON
    pub arguments: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Call ID this result corresponds to
    pub call_id: String,
    
    /// Tool name
    pub name: String,
    
    /// Result content
    pub content: String,
    
    /// Whether execution failed
    pub is_error: bool,
}

/// Tool trait for implementers
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool definition for AI
    fn definition(&self) -> ToolDefinition;
    
    /// Execute the tool
    async fn execute(&self, arguments: serde_json::Value) -> Result<String, ToolError>;
}

/// Registry of available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
    }
    
    /// Get tool definitions for AI
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }
    
    /// Execute a tool call
    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult, ToolError> {
        let tool = self.tools.get(&call.name)
            .ok_or_else(|| ToolError::NotFound(call.name.clone()))?;
        
        let content = tool.execute(call.arguments.clone()).await.map_err(|e| {
            ToolError::ExecutionFailed(e.to_string())
        })?;
        
        Ok(ToolResult {
            call_id: call.id,
            name: call.name,
            content,
            is_error: false,
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in Tools
// ============================================================================

/// Web search tool (stub implementation)
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }
    
    async fn execute(&self, arguments: serde_json::Value) -> Result<String, ToolError> {
        let query = arguments.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing query".into()))?;
        
        // Stub: In production, this would call a real search API
        Ok(format!("[Web Search Results for '{}']\n\n1. Example result 1\n2. Example result 2\n3. Example result 3", query))
    }
}

/// Code execution tool (sandboxed)
pub struct CodeExecuteTool {
    #[allow(dead_code)] // Will be used in sandboxed execution
    timeout_ms: u64,
}

impl CodeExecuteTool {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl Default for CodeExecuteTool {
    fn default() -> Self {
        Self::new(5000)
    }
}

#[async_trait]
impl Tool for CodeExecuteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "code_execute".to_string(),
            description: "Execute code in a sandboxed environment".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["python", "javascript", "rust"],
                        "description": "Programming language"
                    },
                    "code": {
                        "type": "string",
                        "description": "Code to execute"
                    }
                },
                "required": ["language", "code"]
            }),
        }
    }
    
    async fn execute(&self, arguments: serde_json::Value) -> Result<String, ToolError> {
        let language = arguments.get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing language".into()))?;
        
        let code = arguments.get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing code".into()))?;
        
        // Stub: In production, this would run in a real sandbox
        Ok(format!("[Execution Result]\nLanguage: {}\nCode length: {} bytes\nOutput: (sandboxed execution not yet implemented)", language, code.len()))
    }
}

/// File read tool
pub struct FileReadTool {
    base_path: std::path::PathBuf,
}

impl FileReadTool {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new(".")
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_read".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative file path"
                    }
                },
                "required": ["path"]
            }),
        }
    }
    
    async fn execute(&self, arguments: serde_json::Value) -> Result<String, ToolError> {
        let path = arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing path".into()))?;
        
        // Security: Prevent path traversal
        if path.contains("..") || path.starts_with('/') {
            return Err(ToolError::InvalidParameters("invalid path".into()));
        }
        
        let full_path = self.base_path.join(path);
        
        match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                // Limit output size
                if content.len() > 10000 {
                    Ok(format!("{}...\n\n[Truncated - file too large]", &content[..10000]))
                } else {
                    Ok(content)
                }
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!("Failed to read file: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn tool_registry_manages_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(WebSearchTool::new()));
        
        let defs = registry.definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "web_search");
    }
    
    #[tokio::test]
    async fn web_search_returns_results() {
        let tool = WebSearchTool::new();
        let args = serde_json::json!({"query": "rust programming"});
        
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("rust programming"));
    }
    
    #[tokio::test]
    async fn code_execute_validates_params() {
        let tool = CodeExecuteTool::new(5000);
        let args = serde_json::json!({"language": "python"}); // missing code
        
        let result = tool.execute(args).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn file_read_prevents_traversal() {
        let tool = FileReadTool::new("/tmp");
        let args = serde_json::json!({"path": "../../../etc/passwd"});
        
        let result = tool.execute(args).await;
        assert!(result.is_err());
    }
}
