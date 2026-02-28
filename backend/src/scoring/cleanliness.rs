use crate::models::Note;

const MAX_CONSECUTIVE_BLANKS: usize = 1;
const FLAG_TABS_IN_INDENT: bool = true;
const IGNORE_URLS_FOR_LENGTH: bool = true;

pub fn score_cleanliness(code: &str, max_len: usize, notes: &mut Vec<Note>) -> u32 {
    // Instead of subtracting from a flat 100, we accumulate a penalty count and
    // normalize by the total number of lines of the submission.  That way a
    // file with 100 issues in 1000 lines will score the same as one with 10
    // issues in 100 lines; the grade reflects per-line cleanliness rather
    // than absolute issue count.
    let mut penalties: i32 = 0;

    // 0) BOM Check
    let (mut content, has_bom) = if code.starts_with('\u{FEFF}') {
        penalties += 1;
        notes.push(Note {
            line: 1,
            kind: "bom_present".into(),
            severity: "info".into(),
            msg: "File starts with a UTF-8 BOM; prefer no BOM".into(),
        });
        (&code[3..], true) // UTF-8 BOM is 3 bytes
    } else {
        (code, false)
    };

    // 1) EOF Newline Check
    if !code.is_empty() && !code.ends_with('\n') {
        penalties += 1;
        notes.push(Note {
            line: 0, 
            kind: "missing_final_newline".into(),
            severity: "info".into(),
            msg: "File does not end with a newline".into(),
        });
    }

    let mut saw_tab_indent = false;
    let mut saw_space_indent = false;
    let mut consecutive_blanks = 0;
    let mut last_line_idx = 0;
    let mut last_non_blank_line = 0;

    // Use a single pass for all line-based checks
    for (i, line) in content.lines().enumerate() {
        let line_no = i + 1;
        last_line_idx = line_no;

        // 2) Carriage Returns
        if line.ends_with('\r') {
            penalties += 1;
            notes.push(Note {
                line: line_no,
                kind: "carriage_return".into(),
                severity: "info".into(),
                msg: "Line ends with a CR; prefer LF-only".into(),
            });
        }

        let clean_line = line.trim_end_matches('\r');

        // 3) Trailing Whitespace
        if !clean_line.is_empty() && (clean_line.ends_with(' ') || clean_line.ends_with('\t')) {
            penalties += 1;
            notes.push(Note {
                line: line_no,
                kind: "trailing_whitespace".into(),
                severity: "info".into(),
                msg: "Trailing whitespace detected".into(),
            });
        }

        // 4) Line Length (No-allocation URL check)
        let visible_len = clean_line.chars().count();
        if visible_len > max_len {
            if !IGNORE_URLS_FOR_LENGTH || !is_mostly_single_url(clean_line) {
                penalties += 1;
                notes.push(Note {
                    line: line_no,
                    kind: "line_length".into(),
                    severity: "info".into(),
                    msg: format!("Line exceeds {} chars ({} chars)", max_len, visible_len),
                });
            }
        }

        // 5) Indentation
        let indent_part = clean_line.find(|c: char| !c.is_whitespace()).map(|idx| &clean_line[..idx]).unwrap_or(clean_line);
        if !indent_part.is_empty() {
            let contains_tab = indent_part.contains('\t');
            let contains_space = indent_part.contains(' ');
            
            if contains_tab { saw_tab_indent = true; }
            if contains_space { saw_space_indent = true; }

            if FLAG_TABS_IN_INDENT && contains_tab {
                penalties += 1;
                notes.push(Note {
                    line: line_no,
                    kind: "tab_indentation".into(),
                    severity: "info".into(),
                    msg: "Tab character found in indentation".into(),
                });
            }
        }

        // 6) Suspicious Chars
        if let Some((ch, col)) = find_suspicious_control_char(clean_line) {
            penalties += 1;
            notes.push(Note {
                line: line_no,
                kind: "control_character".into(),
                severity: "warn".into(),
                msg: format!("Control character U+{:04X} at col {}", ch as u32, col),
            });
        }

        // 7) TODO/FIXME (Case-insensitive without allocation)
        if contains_marker(clean_line) {
            notes.push(Note {
                line: line_no,
                kind: "todo_marker".into(),
                severity: "info".into(),
                msg: "Found TODO/FIXME marker".into(),
            });
        }

        // 8) Blank Lines
        if clean_line.trim().is_empty() {
            consecutive_blanks += 1;
            if consecutive_blanks > MAX_CONSECUTIVE_BLANKS {
                penalties += 1;
                notes.push(Note {
                    line: line_no,
                    kind: "excess_blank_lines".into(),
                    severity: "info".into(),
                    msg: "Too many consecutive blank lines".into(),
                });
            }
        } else {
            consecutive_blanks = 0;
            last_non_blank_line = line_no;
        }
    }

    // 9) Trailing Blank Lines
    if last_line_idx > last_non_blank_line {
        penalties += 1;
        notes.push(Note {
            line: last_line_idx,
            kind: "trailing_blank_lines".into(),
            severity: "info".into(),
            msg: "Trailing blank lines at EOF".into(),
        });
    }

    // 10) Mixed Indent
    if saw_tab_indent && saw_space_indent {
        penalties += 2;
        notes.push(Note {
            line: 0,
            kind: "mixed_indentation".into(),
            severity: "warn".into(),
            msg: "Mixed tabs and spaces across file".into(),
        });
    }

    // normalise to 0..100 based on line count
    let total_lines = code.lines().count().max(1) as f32;
    let score_f = 100.0 - (penalties as f32 * 100.0 / total_lines);
    score_f.clamp(0.0, 100.0) as u32
}

/* --- Optimized Helpers --- */

/// Case-insensitive check without allocating a new string
fn contains_marker(s: &str) -> bool {
    let s = s.to_ascii_lowercase(); // In modern Rust, this is often optimized, but regex is better for large apps
    s.contains("todo") || s.contains("fixme")
}

fn is_mostly_single_url(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || t.contains(' ') { return false; }
    t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www.")
}

fn find_suspicious_control_char(line: &str) -> Option<(char, usize)> {
    line.char_indices().find_map(|(idx, ch)| {
        let is_bad = (ch < ' ' && ch != '\t') || ch == '\u{7F}' || 
                     matches!(ch, '\u{2028}' | '\u{2029}' | '\u{200B}' | '\u{2060}');
        if is_bad { Some((ch, idx + 1)) } else { None }
    })
}