use std::{
    collections::{HashMap, HashSet}, num::NonZero, path::Path, sync::RwLock
};
use lru::LruCache;

use crate::{
    config::{Rule, RulesConfig},
    errors::Result,
};

/// Thread-safe classifier with dynamic rule updates
pub struct Classifier {
    rules: RwLock<Vec<Rule>>,
    extension_map: RwLock<HashMap<String, Vec<usize>>>,
    regex_rules: RwLock<Vec<usize>>,
    regex_cache: RwLock<LruCache<String, Option<String>>>, // cache filename → destination
}

impl Classifier {
    pub fn new(rules_config: RulesConfig) -> Self {
        let rules = rules_config.rules;
        let (extension_map, regex_rules) = Self::build_indexes(&rules);

        Self {
            rules: RwLock::new(rules),
            extension_map: RwLock::new(extension_map),
            regex_rules: RwLock::new(regex_rules),
            regex_cache: RwLock::new(LruCache::new(NonZero::new(1024).unwrap())), // cache up to 1024 entries
        }
    }

    fn build_indexes(rules: &[Rule]) -> (HashMap<String, Vec<usize>>, Vec<usize>) {
        let mut ext_map: HashMap<String, Vec<usize>> = HashMap::with_capacity(rules.len());
        let mut regex_rules = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            for ext in &rule.extensions {
                ext_map.entry(ext.to_lowercase()).or_default().push(idx);
            }
            if rule.compiled_regex.is_some() {
                regex_rules.push(idx);
            }
        }

        (ext_map, regex_rules)
    }

    pub fn classify<P: AsRef<Path>>(&self, file_path: P) -> Option<String> {
        let rules = self.rules.read().unwrap();
        let extension_map = self.extension_map.read().unwrap();
        let regex_rules = self.regex_rules.read().unwrap();
        let mut regex_cache = self.regex_cache.write().unwrap();

        Self::classify_inner(file_path, &rules, &extension_map, &regex_rules, &mut regex_cache)
    }

    fn classify_inner<P: AsRef<Path>>(
        file_path: P,
        rules: &[Rule],
        extension_map: &HashMap<String, Vec<usize>>,
        regex_rules: &[usize],
        regex_cache: &mut LruCache<String, Option<String>>,
    ) -> Option<String> {
        let path = file_path.as_ref();
        let file_name = path.file_name()?.to_str()?.to_string();

        // Fast path: extension matching
        if let Some(ext) = Self::extract_extension(path) {
            if let Some(rule_indices) = extension_map.get(&ext) {
                let idx = rule_indices[0];
                return Some(rules[idx].destination.clone());
            }
        }

        // Regex cache lookup
        if let Some(cached) = regex_cache.get(&file_name) {
            return cached.clone();
        }

        // Regex matching
        for &idx in regex_rules {
            let rule = &rules[idx];
            if let Some(regex) = &rule.compiled_regex {
                if regex.is_match(&file_name) {
                    let dest = Some(rule.destination.clone());
                    regex_cache.put(file_name, dest.clone());
                    return dest;
                }
            }
        }

        // Cache the miss
        regex_cache.put(file_name, None);
        None
    }

    fn extract_extension(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.trim_start_matches('.').to_lowercase())
    }

    pub fn update_rules(&self, new_config: RulesConfig) -> Result<()> {
        let new_rules = new_config.rules;
        let (new_ext_map, new_regex_rules) = Self::build_indexes(&new_rules);

        *self.rules.write().unwrap() = new_rules;
        *self.extension_map.write().unwrap() = new_ext_map;
        *self.regex_rules.write().unwrap() = new_regex_rules;

        // Clear regex cache (rules changed)
        self.regex_cache.write().unwrap().clear();

        Ok(())
    }

    pub fn classify_batch<P: AsRef<Path>>(&self, files: &[P]) -> Vec<(String, Option<String>)> {
        let rules = self.rules.read().unwrap();
        let extension_map = self.extension_map.read().unwrap();
        let regex_rules = self.regex_rules.read().unwrap();
        let mut regex_cache = self.regex_cache.write().unwrap();

        files
            .iter()
            .map(|file_path| {
                let path_str = file_path.as_ref().to_string_lossy().into_owned();
                let category =
                    Self::classify_inner(file_path, &rules, &extension_map, &regex_rules, &mut regex_cache);
                (path_str, category)
            })
            .collect()
    }

    pub fn get_all_categories(&self) -> HashSet<String> {
        self.rules
            .read()
            .unwrap()
            .iter()
            .map(|rule| rule.category.clone())
            .collect()
    }

    pub fn get_rule_for_category(&self, category: &str) -> Option<Rule> {
        self.rules
            .read()
            .unwrap()
            .iter()
            .find(|rule| rule.category == category)
            .cloned()
    }

    pub fn get_rules_snapshot(&self) -> Vec<Rule> {
        self.rules.read().unwrap().clone()
    }
}
