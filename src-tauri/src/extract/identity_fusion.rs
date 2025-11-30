// Identity Fusion Module
// Entity resolution for passenger names using fuzzy matching and AI analysis

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a canonical person entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonEntity {
    pub id: String,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub confidence: f64,
    pub flight_count: usize,
    pub notes: Option<String>,
}

/// Represents a potential merge candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeCandidate {
    pub source_name: String,
    pub target_entity_id: String,
    pub target_canonical_name: String,
    pub similarity_score: f64,
    pub match_type: MatchType,
    pub auto_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    ExactMatch,
    Abbreviation,      // JE -> Jeffrey Epstein
    Substring,         // JEFFREY -> JEFFREY EPSTEIN
    FuzzyMatch,        // Typo correction
    AIInferred,        // AI determined these are same person
}

/// Configuration for the fusion process
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Minimum Jaro-Winkler similarity for fuzzy matching (0.0-1.0)
    pub fuzzy_threshold: f64,
    /// Auto-merge when similarity exceeds this threshold
    pub auto_merge_threshold: f64,
    /// Known abbreviations to expand
    pub known_abbreviations: HashMap<String, String>,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            fuzzy_threshold: 0.85,
            auto_merge_threshold: 0.95,
            // No pre-defined abbreviations - tool is neutral
            // User defines mappings through the Alias Management UI
            known_abbreviations: HashMap::new(),
        }
    }
}

/// Jaro-Winkler similarity algorithm
/// Returns a score between 0.0 (no similarity) and 1.0 (exact match)
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    let s1 = s1.to_uppercase();
    let s2 = s2.to_uppercase();

    if s1 == s2 {
        return 1.0;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    let match_distance = (len1.max(len2) / 2).saturating_sub(1);

    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];

    let mut matches = 0;
    let mut transpositions = 0;

    // Find matches
    for i in 0..len1 {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(len2);

        for j in start..end {
            if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    // Count transpositions
    let mut k = 0;
    for i in 0..len1 {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let jaro = (matches as f64 / len1 as f64
        + matches as f64 / len2 as f64
        + (matches as f64 - transpositions as f64 / 2.0) / matches as f64)
        / 3.0;

    // Calculate common prefix (up to 4 characters)
    let prefix_len = s1_chars
        .iter()
        .zip(s2_chars.iter())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count();

    // Jaro-Winkler adjustment
    jaro + (prefix_len as f64 * 0.1 * (1.0 - jaro))
}

/// Check if one name is an abbreviation/initials of another
pub fn is_abbreviation(short: &str, long: &str) -> bool {
    let short = short.to_uppercase();
    let long = long.to_uppercase();

    // Remove common titles
    let long = long
        .replace("MR.", "")
        .replace("MR ", "")
        .replace("MS.", "")
        .replace("MS ", "")
        .replace("MRS.", "")
        .replace("MRS ", "")
        .replace("DR.", "")
        .replace("DR ", "")
        .trim()
        .to_string();

    let short_chars: Vec<char> = short.chars().filter(|c| c.is_alphabetic()).collect();
    let long_words: Vec<&str> = long.split_whitespace().collect();

    if short_chars.is_empty() || long_words.is_empty() {
        return false;
    }

    // Check if short matches first letters of long words
    // e.g., "JE" matches "JEFFREY EPSTEIN"
    if short_chars.len() == long_words.len() {
        let initials_match = short_chars
            .iter()
            .zip(long_words.iter())
            .all(|(c, word)| word.starts_with(*c));

        if initials_match {
            return true;
        }
    }

    // Check if short is first name only
    // e.g., "JEFFREY" matches "JEFFREY EPSTEIN"
    if long_words.len() > 1 && long_words[0] == short {
        return true;
    }

    // Check if short is last name only
    if long_words.len() > 1 && long_words.last() == Some(&short.as_str()) {
        return true;
    }

    false
}

/// Check if one string is a substring of another (for partial matches)
pub fn is_partial_match(shorter: &str, longer: &str) -> bool {
    let shorter = shorter.to_uppercase();
    let longer = longer.to_uppercase();

    if shorter.len() >= longer.len() {
        return false;
    }

    // Must be at least 3 chars to be a meaningful partial match
    if shorter.len() < 3 {
        return false;
    }

    longer.contains(&shorter)
}

/// Identity Fusion Engine
pub struct IdentityFusion {
    config: FusionConfig,
    entities: HashMap<String, PersonEntity>,
    name_to_entity: HashMap<String, String>,
}

impl IdentityFusion {
    pub fn new(config: FusionConfig) -> Self {
        Self {
            config,
            entities: HashMap::new(),
            name_to_entity: HashMap::new(),
        }
    }

    /// Create with default config
    pub fn default() -> Self {
        Self::new(FusionConfig::default())
    }

    /// Analyze a list of passenger names and their frequencies
    pub fn analyze_names(&mut self, name_counts: &[(String, usize)]) -> Vec<MergeCandidate> {
        let mut candidates = Vec::new();

        // Sort by frequency (most common first)
        let mut sorted_names: Vec<_> = name_counts.to_vec();
        sorted_names.sort_by(|a, b| b.1.cmp(&a.1));

        // First pass: create entities for high-frequency names
        for (name, count) in &sorted_names {
            if *count >= 5 {
                // High-frequency names become canonical entities
                let normalized = name.to_uppercase().trim().to_string();

                if !self.name_to_entity.contains_key(&normalized) {
                    let entity_id = uuid::Uuid::new_v4().to_string();

                    let entity = PersonEntity {
                        id: entity_id.clone(),
                        canonical_name: normalized.clone(),
                        aliases: vec![normalized.clone()],
                        confidence: 1.0,
                        flight_count: *count,
                        notes: None,
                    };

                    self.entities.insert(entity_id.clone(), entity);
                    self.name_to_entity.insert(normalized, entity_id);
                }
            }
        }

        // Second pass: match remaining names to entities
        for (name, count) in &sorted_names {
            let normalized = name.to_uppercase().trim().to_string();

            if self.name_to_entity.contains_key(&normalized) {
                continue; // Already an entity
            }

            // Check known abbreviations first
            if let Some(expanded) = self.config.known_abbreviations.get(&normalized) {
                let expanded_upper = expanded.to_uppercase();
                if let Some(entity_id) = self.name_to_entity.get(&expanded_upper) {
                    candidates.push(MergeCandidate {
                        source_name: normalized.clone(),
                        target_entity_id: entity_id.clone(),
                        target_canonical_name: expanded_upper,
                        similarity_score: 1.0,
                        match_type: MatchType::Abbreviation,
                        auto_merge: true,
                    });
                    continue;
                }
            }

            // Find best matching entity
            let mut best_match: Option<MergeCandidate> = None;

            for (entity_id, entity) in &self.entities {
                // Check abbreviation
                if is_abbreviation(&normalized, &entity.canonical_name) {
                    let candidate = MergeCandidate {
                        source_name: normalized.clone(),
                        target_entity_id: entity_id.clone(),
                        target_canonical_name: entity.canonical_name.clone(),
                        similarity_score: 0.95,
                        match_type: MatchType::Abbreviation,
                        auto_merge: true,
                    };

                    if best_match.as_ref().map_or(true, |b| candidate.similarity_score > b.similarity_score) {
                        best_match = Some(candidate);
                    }
                    continue;
                }

                // Check partial match (substring)
                if is_partial_match(&normalized, &entity.canonical_name) {
                    let candidate = MergeCandidate {
                        source_name: normalized.clone(),
                        target_entity_id: entity_id.clone(),
                        target_canonical_name: entity.canonical_name.clone(),
                        similarity_score: 0.90,
                        match_type: MatchType::Substring,
                        auto_merge: false, // Require confirmation
                    };

                    if best_match.as_ref().map_or(true, |b| candidate.similarity_score > b.similarity_score) {
                        best_match = Some(candidate);
                    }
                    continue;
                }

                // Check fuzzy match
                let similarity = jaro_winkler_similarity(&normalized, &entity.canonical_name);
                if similarity >= self.config.fuzzy_threshold {
                    let auto_merge = similarity >= self.config.auto_merge_threshold;

                    let candidate = MergeCandidate {
                        source_name: normalized.clone(),
                        target_entity_id: entity_id.clone(),
                        target_canonical_name: entity.canonical_name.clone(),
                        similarity_score: similarity,
                        match_type: MatchType::FuzzyMatch,
                        auto_merge,
                    };

                    if best_match.as_ref().map_or(true, |b| candidate.similarity_score > b.similarity_score) {
                        best_match = Some(candidate);
                    }
                }
            }

            if let Some(candidate) = best_match {
                candidates.push(candidate);
            }
        }

        candidates
    }

    /// Apply a merge - add alias to entity
    pub fn apply_merge(&mut self, candidate: &MergeCandidate) {
        if let Some(entity) = self.entities.get_mut(&candidate.target_entity_id) {
            if !entity.aliases.contains(&candidate.source_name) {
                entity.aliases.push(candidate.source_name.clone());
            }
            self.name_to_entity.insert(candidate.source_name.clone(), candidate.target_entity_id.clone());
        }
    }

    /// Get all entities
    pub fn get_entities(&self) -> Vec<&PersonEntity> {
        self.entities.values().collect()
    }

    /// Export to aliases.json format
    pub fn export_aliases(&self) -> HashMap<String, String> {
        let mut aliases = HashMap::new();

        for entity in self.entities.values() {
            for alias in &entity.aliases {
                aliases.insert(alias.clone(), entity.canonical_name.clone());
            }
        }

        aliases
    }
}

/// Result of identity fusion analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResult {
    pub entities: Vec<PersonEntity>,
    pub merge_candidates: Vec<MergeCandidate>,
    pub unmapped_names: Vec<(String, usize)>,
    pub aliases_map: HashMap<String, String>,
}

/// Prompt for AI to analyze passenger names
pub const AI_IDENTITY_PROMPT: &str = r#"Analyze these passenger names from flight logs and identify which names likely refer to the same person.

INPUT: List of names with flight counts
OUTPUT: JSON groupings

For each group, identify:
1. The canonical (full) name
2. Abbreviations that match (e.g., "JE" = "JEFFREY EPSTEIN")
3. First name only matches (e.g., "JEFFREY" = "JEFFREY EPSTEIN")
4. Typos or OCR errors (e.g., "GHISLANE" = "GHISLAINE")
5. Confidence score (0.0-1.0)

Return JSON format:
{
  "groups": [
    {
      "canonical_name": "JEFFREY EPSTEIN",
      "aliases": ["JE", "JEFFREY", "J. EPSTEIN", "JEFF EPSTEIN"],
      "confidence": 0.95,
      "reasoning": "JE is initials, JEFFREY is first name only"
    }
  ]
}

Names to analyze:
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaro_winkler() {
        assert!((jaro_winkler_similarity("JEFFREY", "JEFFREY") - 1.0).abs() < 0.001);
        assert!(jaro_winkler_similarity("JEFFREY EPSTEIN", "JEFFREY EPSTIEN") > 0.95);
        assert!(jaro_winkler_similarity("GHISLAINE", "GHISLANE") > 0.90);
        assert!(jaro_winkler_similarity("SARAH", "JOHN") < 0.70);
    }

    #[test]
    fn test_is_abbreviation() {
        assert!(is_abbreviation("JE", "JEFFREY EPSTEIN"));
        assert!(is_abbreviation("GM", "GHISLAINE MAXWELL"));
        assert!(is_abbreviation("JEFFREY", "JEFFREY EPSTEIN"));
        assert!(!is_abbreviation("JM", "JEFFREY EPSTEIN"));
        assert!(!is_abbreviation("SARAH", "JOHN DOE"));
    }

    #[test]
    fn test_is_partial_match() {
        assert!(is_partial_match("JEFFREY", "JEFFREY EPSTEIN"));
        assert!(!is_partial_match("JEFFREY EPSTEIN", "JEFFREY"));
        assert!(!is_partial_match("JE", "JEFFREY EPSTEIN")); // Too short
    }

    #[test]
    fn test_fusion_basic() {
        let mut fusion = IdentityFusion::default();

        let names = vec![
            ("JEFFREY EPSTEIN".to_string(), 100),
            ("JE".to_string(), 50),
            ("JEFFREY".to_string(), 30),
            ("GHISLAINE MAXWELL".to_string(), 80),
            ("GM".to_string(), 40),
        ];

        let candidates = fusion.analyze_names(&names);

        // Should find JE -> JEFFREY EPSTEIN
        assert!(candidates.iter().any(|c|
            c.source_name == "JE" && c.target_canonical_name == "JEFFREY EPSTEIN"
        ));

        // Should find GM -> GHISLAINE MAXWELL
        assert!(candidates.iter().any(|c|
            c.source_name == "GM" && c.target_canonical_name == "GHISLAINE MAXWELL"
        ));
    }
}
