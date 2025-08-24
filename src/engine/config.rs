use serde::Deserialize;
use regex::Regex;
use std::{fs, path::Path};

use crate::errors::{FileOrganizerError, Result};

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub category: String,

    #[serde(default)]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub regex: Option<String>,

    pub destination: String,

    // Compiled regex (not from JSON, built at runtime)
    #[serde(skip)]
    pub compiled_regex: Option<Regex>,
}

#[derive(Debug, Deserialize)]
pub struct RulesConfig {
    pub rules: Vec<Rule>,
}

impl RulesConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = fs::read_to_string(&path)
            .map_err(|e| FileOrganizerError::Io(e))?;

        let mut config: RulesConfig = serde_json::from_str(&data)
            .map_err(|e| FileOrganizerError::Json {
                path: path.as_ref().to_path_buf(),
                source: e
            })?;

        // Compile regex patterns for faster matching
        for (idx, rule) in config.rules.iter_mut().enumerate() {
            if rule.extensions.is_empty() && rule.regex.is_none() {
                return Err(FileOrganizerError::InvalidRule(format!(
                    "Rule '{}' (index {}) must have at least one of 'extensions' or 'regex'",
                    rule.category,
                    idx + 1
                )));
            }
            

            // normalize all extensions to lowercase without leading dot
            rule.extensions = rule.extensions
                .iter()
                .map(|e| e.trim_start_matches('.').to_lowercase())
                .collect();

            if let Some(pattern) = &rule.regex {
                let compiled = Regex::new(&pattern)
                    .map_err(|e| FileOrganizerError::Regex {
                        pattern: pattern.clone(),
                        source: e,
                    })?;
                rule.compiled_regex = Some(compiled);
            }
        }

        Ok(config)
    }
}