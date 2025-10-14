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
        #[serde(default)]
        default: bool,
    },

    /// Numeric input option
    Number {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default)]
        default: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },

    /// Text input option
    Text {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default)]
        default: String,
    },

    /// Dropdown selection option
    Select {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: String,
        options: Vec<SelectOption>,
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
    /// Get the default value for this config option
    pub fn get_default_value(&self) -> serde_json::Value {
        match self {
            ConfigOption::CheckBox { default, .. } => serde_json::json!(default),
            ConfigOption::Number { default, .. } => serde_json::json!(default),
            ConfigOption::Text { default, .. } => serde_json::json!(default),
            ConfigOption::Select { default, .. } => serde_json::json!(default),
        }
    }

    /// Get the ID of this config option
    pub fn id(&self) -> &str {
        match self {
            ConfigOption::CheckBox { id, .. } => id,
            ConfigOption::Number { id, .. } => id,
            ConfigOption::Text { id, .. } => id,
            ConfigOption::Select { id, .. } => id,
        }
    }
}

impl ModConfig {
    /// Generate default user configuration from config options
    pub fn generate_default_config(&self) -> UserConfig {
        let mut config = UserConfig::new();

        for option in &self.config {
            config.insert(option.id().to_string(), option.get_default_value());
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
