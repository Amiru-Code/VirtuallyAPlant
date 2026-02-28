use axum::http::StatusCode;
use crate::models::{JudgeResp, Note};

pub mod language;
mod cleanliness;
mod correctness;
mod structure;

pub use language::Language;

/// Compute every category score and also append human-readable advice notes
/// based on twenty‑point intervals.  Each component now returns a per-line
/// averaged score (0..100) so that longer submissions are judged by the
/// density of issues rather than the absolute count.  The score-to-advice
/// mapping lives in `add_insight`, which pushes an extra note indicating what
/// the student should work on.
pub fn compute_all(code: &str, lang: Language, max_line_len: usize) -> (JudgeResp, StatusCode) {
    let mut notes: Vec<Note> = Vec::new();

    let c1 = cleanliness::score_cleanliness(code, max_line_len, &mut notes);
    let c2 = correctness::score_correctness(code, &lang, &mut notes);
    let c3 = structure::score_structure(code, &mut notes);

    // add range‑based advice notes for every component
    add_insight(c1, "cleanliness", &mut notes);
    add_insight(c2, "correctness", &mut notes);
    add_insight(c3, "structure", &mut notes);

    let overall = ((c1 + c2 + c3) / 3).clamp(0, 100);

    (
        JudgeResp { cleanliness: c1, correctness: c2, structure: c3, overall, notes },
        StatusCode::OK,
    )
}

/// Convert a numerical score into a prose recommendation and append it as a
/// note.  Each twenty‑point bucket gets its own message so that callers of the
/// API receive a quick hint about what to improve.
fn add_insight(score: u32, category: &str, notes: &mut Vec<Note>) {
    let advice = match category {
        "cleanliness" => match score {
            0..=19 => "Cleanliness is critically low; lots of trailing whitespace,
                        inconsistent indentation, or extremely long lines.".to_string(),
            20..=39 => "Poor cleanliness: fix trailing spaces and respect the max
                        line length.".to_string(),
            40..=59 => "Fair cleanliness; consider removing duplicate blank lines
                        and normalizing indentation.".to_string(),
            60..=79 => "Good cleanliness; keep lines short and avoid needless
                        whitespace.".to_string(),
            80..=100 => "Excellent cleanliness! Minor polish only.".to_string(),
            _ => "Cleanliness score uncertain.".to_string(),
        },
        "correctness" => match score {
            0..=19 => "Correctness very low: syntax errors or major logic issues.
                        Run the code and address all compile/runtime errors.".to_string(),
            20..=39 => "Low correctness: unbalanced braces/tags or missing
                        semicolons detected.".to_string(),
            40..=59 => "Fair correctness; watch for edge cases and simple
                        logic mistakes.".to_string(),
            60..=79 => "Good correctness; a few small logical issues remain.".to_string(),
            80..=100 => "Excellent correctness; code appears to behave as expected.".to_string(),
            _ => format!("Correctness score {}", score),
        }.to_string(),
        "structure" => match score {
            0..=19 => "Structure very poor: duplicate lines and haphazard
                        organization.".to_string(),
            20..=39 => "Weak structure: too many small functions or repeated code.".to_string(),
            40..=59 => "Fair structure; consider refactoring and extracting
                        common logic.".to_string(),
            60..=79 => "Good structure; a little more consolidation would help.".to_string(),
            80..=100 => "Excellent structure; code is cleanly organized.".to_string(),
            _ => format!("Structure score {}", score),
        }.to_string(),
        _ => format!("{} score: {}", category, score),
    };

    notes.push(Note {
        line: 0,
        kind: format!("{}_advice", category),
        severity: "info".into(),
        msg: advice,
    });
}