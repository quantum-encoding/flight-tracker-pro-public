// OCR Learning System - Pattern recognition and correction suggestions

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrCorrection {
    pub id: String,
    pub user_id: String,
    pub field_name: String,
    pub original_value: Option<String>,
    pub corrected_value: String,
    pub image_hash: Option<String>,
    pub confidence_score: Option<f64>,
    pub pattern_type: Option<String>,
    pub applied_count: i32,
    pub verified: bool,
    pub created_at: String,
    pub last_applied: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrLearningPattern {
    pub id: String,
    pub user_id: String,
    pub pattern_type: String,
    pub field_name: String,
    pub match_pattern: String,
    pub replacement_value: String,
    pub confidence: f64,
    pub occurrence_count: i32,
    pub success_count: i32,
    pub rejection_count: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionSuggestion {
    pub field_name: String,
    pub original_value: String,
    pub suggested_value: String,
    pub confidence: f64,
    pub pattern_type: String,
    pub reason: String,
}

/// Record a user correction to learn from
pub fn record_correction(
    conn: &rusqlite::Connection,
    user_id: &str,
    field_name: &str,
    original_value: Option<&str>,
    corrected_value: &str,
    image_hash: Option<&str>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();

    // Infer pattern type
    let pattern_type = infer_pattern_type(field_name, original_value, corrected_value);

    conn.execute(
        "INSERT INTO ocr_corrections (
            id, user_id, field_name, original_value, corrected_value,
            image_hash, pattern_type, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
        rusqlite::params![
            id,
            user_id,
            field_name,
            original_value,
            corrected_value,
            image_hash,
            pattern_type,
        ],
    )?;

    // Update or create learning pattern
    update_learning_patterns(conn, user_id, field_name, original_value, corrected_value, &pattern_type)?;

    Ok(id)
}

/// Infer what type of pattern this correction represents
fn infer_pattern_type(field_name: &str, original: Option<&str>, corrected: &str) -> String {
    if let Some(orig) = original {
        // Airport codes
        if field_name.contains("airport") && orig.len() <= 4 && corrected.len() <= 4 {
            return "airport_code".to_string();
        }

        // Abbreviation expansion (short to long)
        if orig.len() < corrected.len() && corrected.contains(' ') {
            return "abbreviation".to_string();
        }

        // Format fix (same length, similar characters)
        if orig.len() == corrected.len() {
            let similarity = similarity_score(orig, corrected);
            if similarity > 0.7 {
                return "ocr_misread".to_string();
            }
        }

        // Date/time format
        if field_name.contains("datetime") || field_name.contains("date") {
            return "format_fix".to_string();
        }
    }

    "substitution".to_string()
}

/// Calculate simple similarity score between two strings
fn similarity_score(s1: &str, s2: &str) -> f64 {
    let len = s1.len().max(s2.len());
    if len == 0 {
        return 1.0;
    }

    let matching_chars = s1.chars()
        .zip(s2.chars())
        .filter(|(a, b)| a == b)
        .count();

    matching_chars as f64 / len as f64
}

/// Update or create learning patterns based on a correction
fn update_learning_patterns(
    conn: &rusqlite::Connection,
    user_id: &str,
    field_name: &str,
    original_value: Option<&str>,
    corrected_value: &str,
    pattern_type: &str,
) -> Result<()> {
    if let Some(orig) = original_value {
        // Check if pattern exists
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM ocr_learning_patterns
                 WHERE user_id = ?1 AND field_name = ?2
                 AND match_pattern = ?3 AND pattern_type = ?4",
                rusqlite::params![user_id, field_name, orig, pattern_type],
                |row| row.get(0),
            )
            .ok();

        if let Some(pattern_id) = existing {
            // Update existing pattern
            conn.execute(
                "UPDATE ocr_learning_patterns
                 SET occurrence_count = occurrence_count + 1,
                     confidence = MIN(1.0, confidence + 0.05),
                     updated_at = datetime('now')
                 WHERE id = ?1",
                rusqlite::params![pattern_id],
            )?;
        } else {
            // Create new pattern
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO ocr_learning_patterns (
                    id, user_id, pattern_type, field_name, match_pattern,
                    replacement_value, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))",
                rusqlite::params![
                    id,
                    user_id,
                    pattern_type,
                    field_name,
                    orig,
                    corrected_value,
                ],
            )?;
        }
    }

    Ok(())
}

/// Get all active learning patterns for a user
pub fn get_active_patterns(
    conn: &rusqlite::Connection,
    user_id: &str,
) -> Result<Vec<OcrLearningPattern>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, pattern_type, field_name, match_pattern, replacement_value,
                confidence, occurrence_count, success_count, rejection_count, is_active,
                created_at, updated_at, last_used
         FROM ocr_learning_patterns
         WHERE user_id = ?1 AND is_active = 1
         ORDER BY confidence DESC, occurrence_count DESC"
    )?;

    let patterns = stmt
        .query_map([user_id], |row| {
            Ok(OcrLearningPattern {
                id: row.get(0)?,
                user_id: row.get(1)?,
                pattern_type: row.get(2)?,
                field_name: row.get(3)?,
                match_pattern: row.get(4)?,
                replacement_value: row.get(5)?,
                confidence: row.get(6)?,
                occurrence_count: row.get(7)?,
                success_count: row.get(8)?,
                rejection_count: row.get(9)?,
                is_active: row.get::<_, i32>(10)? == 1,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                last_used: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(patterns)
}

/// Apply learned patterns to OCR results and generate suggestions
pub fn apply_patterns_to_ocr_result(
    conn: &rusqlite::Connection,
    user_id: &str,
    field_values: &HashMap<String, Option<String>>,
) -> Result<Vec<CorrectionSuggestion>> {
    let patterns = get_active_patterns(conn, user_id)?;
    let mut suggestions = Vec::new();

    for (field_name, value_opt) in field_values {
        if let Some(value) = value_opt {
            // Find matching patterns for this field
            for pattern in patterns.iter().filter(|p| &p.field_name == field_name) {
                // Exact match
                if &pattern.match_pattern == value {
                    suggestions.push(CorrectionSuggestion {
                        field_name: field_name.clone(),
                        original_value: value.clone(),
                        suggested_value: pattern.replacement_value.clone(),
                        confidence: pattern.confidence,
                        pattern_type: pattern.pattern_type.clone(),
                        reason: format!(
                            "Learned from {} previous correction(s)",
                            pattern.occurrence_count
                        ),
                    });
                }
                // Fuzzy match for OCR misreads
                else if pattern.pattern_type == "ocr_misread" {
                    let similarity = similarity_score(value, &pattern.match_pattern);
                    if similarity > 0.8 {
                        suggestions.push(CorrectionSuggestion {
                            field_name: field_name.clone(),
                            original_value: value.clone(),
                            suggested_value: pattern.replacement_value.clone(),
                            confidence: pattern.confidence * similarity,
                            pattern_type: pattern.pattern_type.clone(),
                            reason: format!(
                                "Similar to known correction ({}% match)",
                                (similarity * 100.0) as i32
                            ),
                        });
                    }
                }
            }
        }
    }

    // Sort by confidence
    suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    Ok(suggestions)
}

/// Mark a suggestion as accepted (increase confidence)
pub fn accept_suggestion(
    conn: &rusqlite::Connection,
    user_id: &str,
    field_name: &str,
    match_pattern: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ocr_learning_patterns
         SET success_count = success_count + 1,
             confidence = MIN(1.0, confidence + 0.1),
             last_used = datetime('now'),
             updated_at = datetime('now')
         WHERE user_id = ?1 AND field_name = ?2 AND match_pattern = ?3",
        rusqlite::params![user_id, field_name, match_pattern],
    )?;

    Ok(())
}

/// Mark a suggestion as rejected (decrease confidence)
pub fn reject_suggestion(
    conn: &rusqlite::Connection,
    user_id: &str,
    field_name: &str,
    match_pattern: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ocr_learning_patterns
         SET rejection_count = rejection_count + 1,
             confidence = MAX(0.1, confidence - 0.15),
             updated_at = datetime('now')
         WHERE user_id = ?1 AND field_name = ?2 AND match_pattern = ?3",
        rusqlite::params![user_id, field_name, match_pattern],
    )?;

    // Disable pattern if confidence drops too low or rejections exceed successes significantly
    conn.execute(
        "UPDATE ocr_learning_patterns
         SET is_active = 0
         WHERE user_id = ?1 AND field_name = ?2 AND match_pattern = ?3
         AND (confidence < 0.2 OR rejection_count > success_count + 3)",
        rusqlite::params![user_id, field_name, match_pattern],
    )?;

    Ok(())
}

/// Get correction history for a user
pub fn get_correction_history(
    conn: &rusqlite::Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<OcrCorrection>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, field_name, original_value, corrected_value,
                image_hash, confidence_score, pattern_type, applied_count, verified,
                created_at, last_applied
         FROM ocr_corrections
         WHERE user_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2"
    )?;

    let corrections = stmt
        .query_map(rusqlite::params![user_id, limit], |row| {
            Ok(OcrCorrection {
                id: row.get(0)?,
                user_id: row.get(1)?,
                field_name: row.get(2)?,
                original_value: row.get(3)?,
                corrected_value: row.get(4)?,
                image_hash: row.get(5)?,
                confidence_score: row.get(6)?,
                pattern_type: row.get(7)?,
                applied_count: row.get(8)?,
                verified: row.get::<_, i32>(9)? == 1,
                created_at: row.get(10)?,
                last_applied: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(corrections)
}
