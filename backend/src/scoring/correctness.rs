use crate::models::Note;
use super::language::Language;

pub fn score_correctness(code: &str, lang: &Language, notes: &mut Vec<Note>) -> u32 {
    // The correctness module originally started at 100 and added/subtracted
    // deltas from various heuristics.  To make it fairer across long vs short
    // submissions we compute the raw score then convert it into an "errors per
    // line" penalty and rescale to 0..100, similar to the other categories.
    let mut score: i32 = 100;

    let open = code.matches('{').count();
    let close = code.matches('}').count();
    if open != close {
        score -= 10;
        notes.push(Note { line: 0, kind: "brace_balance".into(), severity: "warn".into(), msg: format!("Unbalanced braces: open {} vs close {}", open, close) });
    }

    match lang {
        Language::Html => {
            let opens = code.matches("<div").count();
            let closes = code.matches("</div>").count();
            if opens != closes {
                score -= 8;
                notes.push(Note { line: 0, kind: "html_div_balance".into(), severity: "warn".into(), msg: format!("<div> count {} != </div> count {}", opens, closes) });
            }
        }
        Language::Css => {
            if !code.trim().is_empty() && !code.trim().ends_with('}') {
                score -= 5;
                notes.push(Note { line: 0, kind: "css_block_end".into(), severity: "info".into(), msg: "CSS seems not to end with a '}'".into() });
            }
        }
        Language::Javascript => {
            for (i, line) in code.lines().enumerate() {
                let t = line.trim_end();
                if t.is_empty() || t.starts_with("//") || t.starts_with("/*") { continue; }
                if !(t.ends_with(';') || t.ends_with('{') || t.ends_with('}') || t.ends_with(',') || t.ends_with(':')) {
                    score -= 1;
                    notes.push(Note { line: i + 1, kind: "js_semicolon".into(), severity: "info".into(), msg: "Potential missing semicolon (heuristic)".into() });
                }
            }
        }
        _ => {}
    }

    // clamp first, then compute normalized value based on line count
    let raw = score.clamp(0, 100) as i32;
    let errors = 100 - raw; // how far below perfect we are; positive means issues
    let total_lines = code.lines().count().max(1) as f32;
    let score_f = 100.0 - (errors as f32 * 100.0 / total_lines);
    score_f.clamp(0.0, 100.0) as u32
}
