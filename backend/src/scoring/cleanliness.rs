use crate::models::Note;

pub fn score_cleanliness(code: &str, max_len: usize, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;
    for (i, line) in code.lines().enumerate() {
        let line_no = i + 1;
        if line.ends_with(' ') || line.ends_with('\t') {
            score -= 1;
            notes.push(Note { line: line_no, kind: "trailing_whitespace".into(), severity: "info".into(), msg: "Trailing whitespace".into() });
        }
        if line.chars().count() > max_len {
            score -= 1;
            notes.push(Note { line: line_no, kind: "line_length".into(), severity: "info".into(), msg: format!("Line exceeds {} characters", max_len) });
        }
    }
    score.clamp(0, 100) as u32
}