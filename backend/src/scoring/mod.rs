use axum::http::StatusCode;
use crate::models::{JudgeResp, Note};

pub mod language;
mod cleanliness;
mod correctness;
mod structure;

pub use language::Language;

pub fn compute_all(code: &str, lang: Language, max_line_len: usize) -> (JudgeResp, StatusCode) {
    let mut notes: Vec<Note> = Vec::new();

    let c1 = cleanliness::score_cleanliness(code, max_line_len, &mut notes);
    let c2 = correctness::score_correctness(code, &lang, &mut notes);
    let c3 = structure::score_structure(code, &mut notes);

    let overall = ((c1 + c2 + c3) / 3).clamp(0, 100);

    (
        JudgeResp { cleanliness: c1, correctness: c2, structure: c3, overall, notes },
        StatusCode::OK,
    )
}