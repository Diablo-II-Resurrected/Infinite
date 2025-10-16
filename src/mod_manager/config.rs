use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mod configuration from mod.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModConfig {
    /// Mod name
    pub name: String,

    /// Mod description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Mod author (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Mod website URL (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,

    /// Mod version
    pub version: String,

    /// Configuration options for the user
    #[serde(default)]
    pub config: Vec<ConfigOption>,
}

/// Configuration option types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConfigOption {
    /// Boolean checkbox option
    #[serde(rename = "checkbox")]
    CheckBox {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, alias = "defaultValue")]
        default: bool,
    },

    /// Numeric input option
    Number {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, alias = "defaultValue")]
        default: f64,
        #[serde(skip_serializing_if = "Option::is_none", alias = "minValue")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none", alias = "maxValue")]
        max: Option<f64>,
    },

    /// Text input option
    Text {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, alias = "defaultValue")]
        default: String,
    },

    /// Dropdown selection option
    Select {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(alias = "defaultValue")]
        default: String,
        options: Vec<SelectOption>,
    },

    /// Section header (for UI organization, no value)
    Section {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none", alias = "defaultExpanded")]
        default_expanded: Option<bool>,
    },
}

/// Option for select dropdown
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// User configuration values
pub type UserConfig = HashMap<String, serde_json::Value>;

impl ConfigOption {
    /// Get the default value for this config option (returns None for Section)
    pub fn get_default_value(&self) -> Option<serde_json::Value> {
        match self {
            ConfigOption::CheckBox { default, .. } => Some(serde_json::json!(default)),
            ConfigOption::Number { default, .. } => Some(serde_json::json!(default)),
            ConfigOption::Text { default, .. } => Some(serde_json::json!(default)),
            ConfigOption::Select { default, .. } => Some(serde_json::json!(default)),
            ConfigOption::Section { .. } => None, // Sections don't have values
        }
    }

    /// Get the ID of this config option
    pub fn id(&self) -> &str {
        match self {
            ConfigOption::CheckBox { id, .. } => id,
            ConfigOption::Number { id, .. } => id,
            ConfigOption::Text { id, .. } => id,
            ConfigOption::Select { id, .. } => id,
            ConfigOption::Section { id, .. } => id,
        }
    }
}

impl ModConfig {
    /// Generate default user configuration from config options
    pub fn generate_default_config(&self) -> UserConfig {
        let mut config = UserConfig::new();

        for option in &self.config {
            // Only add options that have values (skip sections)
            if let Some(value) = option.get_default_value() {
                config.insert(option.id().to_string(), value);
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mod_config() {
        let json = r#"
        {
            "name": "Test Mod",
            "description": "A test mod",
            "author": "Test Author",
            "version": "1.0",
            "config": [
                {
                    "type": "checkbox",
                    "id": "enabled",
                    "name": "Enable Feature",
                    "default": true
                },
                {
                    "type": "number",
                    "id": "value",
                    "name": "Value",
                    "default": 100,
                    "min": 1,
                    "max": 1000
                }
            ]
        }
        "#;

        let config: ModConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "Test Mod");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.config.len(), 2);

        let defaults = config.generate_default_config();
        assert_eq!(defaults.get("enabled").unwrap(), &serde_json::json!(true));
        assert_eq!(defaults.get("value").unwrap(), &serde_json::json!(100.0));
    }
}
