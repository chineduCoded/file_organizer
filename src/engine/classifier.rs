use std::path::Path;

use crate::config::RulesConfig;



pub struct Classifier {
    rules: RulesConfig,
}

impl Classifier {
    pub fn new(rules: RulesConfig) -> Self {
        Self  { rules }
    }

    pub fn classify<P: AsRef<Path>>(&self, file: P) -> Option<&str> {
        let filename = file.as_ref().file_name()?.to_str()?.to_lowercase();
        let ext = file.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        for rule in &self.rules.rules {
            // Extension-based matching
            if let Some(ref ext) = ext {
                if rule.extensions.iter().any(|e| e == ext) {
                    return Some(&rule.destination);
                }
            }

            // Regex-based matching
            if let Some(regex) = &rule.compiled_regex {
                if regex.is_match(&filename) {
                    return Some(&rule.destination)
                }
            }
        }

        None
    }
}