use std::collections::HashSet;
use crate::models::Note;

pub fn score_structure(code: &str, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;

    let lines: Vec<&str> = code.lines().collect();
    let unique: HashSet<&str> = lines.iter().copied().collect();
    if unique.len() < lines.len() {
        score -= 5;
        notes.push(Note { line: 0, kind: "duplication".into(), severity: "info".into(), msg: "Duplicate lines detected".into() });
    }

    let func_like = ["function", "fn "];
    let func_count: usize = func_like.iter().map(|k| code.matches(k).count()).sum();
    if func_count > 25 {
        score -= 5;
        notes.push(Note { line: 0, kind: "function_density".into(), severity: "info".into(), msg: format!("High function-like count ({})", func_count) });
    }

    score.clamp(0, 100) as u32
}