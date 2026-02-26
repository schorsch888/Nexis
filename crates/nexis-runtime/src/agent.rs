//! Agent role configuration and registry.
//!
//! This module loads role files from `.nexis/agents` and composes
//! a stable identity prompt that can be prepended to user input.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Agent role configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Human-readable name.
    pub name: String,
    /// Core role statement.
    pub role: String,
    /// Capability list.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Communication vibe.
    pub vibe: String,
    /// Hard constraints.
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// Agent registry errors.
#[derive(Debug, Error)]
pub enum AgentRegistryError {
    #[error("agent directory does not exist: {0}")]
    DirectoryMissing(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid agent file extension: {0}")]
    InvalidExtension(String),
    #[error("failed to parse `{path}`: {message}")]
    Parse { path: String, message: String },
}

/// Registry for configured agents.
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    dir: PathBuf,
    agents: HashMap<String, AgentConfig>,
}

impl AgentRegistry {
    /// Build and load a registry from a specific directory.
    pub fn from_dir(dir: impl Into<PathBuf>) -> Result<Self, AgentRegistryError> {
        let dir = dir.into();
        let mut registry = Self {
            dir,
            agents: HashMap::new(),
        };
        registry.reload()?;
        Ok(registry)
    }

    /// Build and load a registry from a workspace root.
    pub fn from_workspace_root(root: impl AsRef<Path>) -> Result<Self, AgentRegistryError> {
        Self::from_dir(root.as_ref().join(".nexis").join("agents"))
    }

    /// Reload all agent files from disk.
    pub fn reload(&mut self) -> Result<(), AgentRegistryError> {
        if !self.dir.exists() {
            return Err(AgentRegistryError::DirectoryMissing(
                self.dir.display().to_string(),
            ));
        }

        self.agents.clear();
        for entry in
            fs::read_dir(&self.dir).map_err(|err| AgentRegistryError::Io(err.to_string()))?
        {
            let entry = entry.map_err(|err| AgentRegistryError::Io(err.to_string()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("yaml" | "yml" | "json")
            ) {
                continue;
            }

            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_string();
            let config = load_agent_file(&path)?;
            self.agents.insert(id, config);
        }
        Ok(())
    }

    /// Return configured directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// List available agent IDs.
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.agents.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Fetch an agent config by id or name.
    pub fn get(&self, id_or_name: &str) -> Option<&AgentConfig> {
        if let Some(config) = self.agents.get(id_or_name) {
            return Some(config);
        }
        self.agents
            .values()
            .find(|config| config.name.eq_ignore_ascii_case(id_or_name))
    }
}

/// Load one agent file.
pub fn load_agent_file(path: impl AsRef<Path>) -> Result<AgentConfig, AgentRegistryError> {
    let path = path.as_ref();
    let content =
        fs::read_to_string(path).map_err(|err| AgentRegistryError::Io(err.to_string()))?;
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    match ext {
        "yaml" | "yml" => parse_yaml_agent(&content).map_err(|message| AgentRegistryError::Parse {
            path: path.display().to_string(),
            message,
        }),
        "json" => {
            serde_json::from_str::<AgentConfig>(&content).map_err(|err| AgentRegistryError::Parse {
                path: path.display().to_string(),
                message: err.to_string(),
            })
        }
        _ => Err(AgentRegistryError::InvalidExtension(
            path.display().to_string(),
        )),
    }
}

fn parse_yaml_agent(content: &str) -> Result<AgentConfig, String> {
    let mut scalars: HashMap<String, String> = HashMap::new();
    let mut lists: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_list_key: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            if let Some(key) = current_list_key.as_ref() {
                lists
                    .entry(key.clone())
                    .or_default()
                    .push(clean_yaml_value(item));
                continue;
            }
            return Err(format!("list item without list key: `{trimmed}`"));
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(format!("unsupported nested yaml line: `{line}`"));
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("invalid yaml line: `{line}`"));
        };
        let key = key.trim().to_string();
        let value = value.trim();
        if value.is_empty() {
            current_list_key = Some(key.clone());
            lists.entry(key).or_default();
        } else {
            current_list_key = None;
            scalars.insert(key, clean_yaml_value(value));
        }
    }

    let name = scalars
        .remove("name")
        .ok_or_else(|| "missing required field `name`".to_string())?;
    let role = scalars
        .remove("role")
        .ok_or_else(|| "missing required field `role`".to_string())?;
    let vibe = scalars
        .remove("vibe")
        .ok_or_else(|| "missing required field `vibe`".to_string())?;

    Ok(AgentConfig {
        name,
        role,
        skills: lists.remove("skills").unwrap_or_default(),
        vibe,
        constraints: lists.remove("constraints").unwrap_or_default(),
    })
}

fn clean_yaml_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

/// Compose agent identity prompt plus user prompt.
pub fn compose_agent_prompt(agent: &AgentConfig, user_prompt: &str) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are an AI agent with the following identity.\n");
    prompt.push_str(&format!("Name: {}\n", agent.name));
    prompt.push_str(&format!("Role: {}\n", agent.role));
    prompt.push_str(&format!("Vibe: {}\n", agent.vibe));

    if !agent.skills.is_empty() {
        prompt.push_str("Skills:\n");
        for skill in &agent.skills {
            prompt.push_str(&format!("- {}\n", skill));
        }
    }

    if !agent.constraints.is_empty() {
        prompt.push_str("Constraints:\n");
        for constraint in &agent.constraints {
            prompt.push_str(&format!("- {}\n", constraint));
        }
    }

    prompt.push_str("\nUser request:\n");
    prompt.push_str(user_prompt);
    prompt
}

#[cfg(test)]
mod tests {
    use super::{compose_agent_prompt, load_agent_file, AgentRegistry};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("nexis-agent-test-{suffix}-{nanos}"));
        fs::create_dir_all(&path).expect("should create temp dir");
        path
    }

    #[test]
    fn loads_yaml_agent_file() {
        let dir = temp_dir("yaml");
        let file = dir.join("coder.yaml");
        fs::write(
            &file,
            r#"
name: Coder
role: Full-Stack Product Engineer
skills:
  - Rust
  - TypeScript
vibe: Professional
constraints:
  - no fluff
"#,
        )
        .expect("should write file");

        let config = load_agent_file(&file).expect("yaml should parse");
        assert_eq!(config.name, "Coder");
        assert_eq!(config.role, "Full-Stack Product Engineer");
        assert_eq!(config.skills.len(), 2);
    }

    #[test]
    fn loads_json_agent_file() {
        let dir = temp_dir("json");
        let file = dir.join("reviewer.json");
        fs::write(
            &file,
            r#"{
  "name": "Reviewer",
  "role": "Code Reviewer",
  "skills": ["Code review"],
  "vibe": "Strict",
  "constraints": ["Be direct"]
}"#,
        )
        .expect("should write file");

        let config = load_agent_file(&file).expect("json should parse");
        assert_eq!(config.name, "Reviewer");
        assert_eq!(config.role, "Code Reviewer");
        assert_eq!(config.constraints, vec!["Be direct"]);
    }

    #[test]
    fn registry_lists_and_fetches_agents() {
        let dir = temp_dir("registry");
        fs::write(
            dir.join("workspace-coder.yaml"),
            r#"
name: Workspace Coder
role: Product Engineer
skills: []
vibe: Fast
constraints: []
"#,
        )
        .expect("should write file");
        fs::write(
            dir.join("architect.json"),
            r#"{"name":"Architect","role":"System Architect","skills":[],"vibe":"Pragmatic","constraints":[]}"#,
        )
        .expect("should write file");

        let registry = AgentRegistry::from_dir(&dir).expect("registry should load");
        let ids = registry.list();
        assert_eq!(
            ids,
            vec!["architect".to_string(), "workspace-coder".to_string()]
        );
        assert!(registry.get("workspace-coder").is_some());
        assert!(registry.get("Architect").is_some());
    }

    #[test]
    fn prompt_contains_identity_sections() {
        let dir = temp_dir("prompt");
        let file = dir.join("tester.yaml");
        fs::write(
            &file,
            r#"
name: Tester
role: QA Engineer
skills:
  - Test design
vibe: Thorough
constraints:
  - Evidence first
"#,
        )
        .expect("should write file");
        let agent = load_agent_file(file).expect("should parse");
        let prompt = compose_agent_prompt(&agent, "Write test cases");

        assert!(prompt.contains("Name: Tester"));
        assert!(prompt.contains("Role: QA Engineer"));
        assert!(prompt.contains("User request:\nWrite test cases"));
    }
}
