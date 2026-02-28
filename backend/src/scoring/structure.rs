use std::collections::{HashMap, HashSet};
use crate::models::Note;

pub fn score_structure(code: &str, notes: &mut Vec<Note>) -> u32 {
    // similar to cleanliness we count penalties and later divide by total lines
    let mut penalties: i32 = 0;

    // ---------------------------
    // 0) Basic line preprocessing
    // ---------------------------
    let raw_lines: Vec<&str> = code.lines().collect();
    let total_lines = raw_lines.len();

    // For normalization: trim right, collapse internal whitespace lightly for duplicate checks
    let mut norm_lines: Vec<String> = Vec::with_capacity(total_lines);
    norm_lines.extend(raw_lines.iter().map(|l| normalize_line_for_dups(l)));

    // Heuristic comment/blank detection (lightweight & language-agnostic-ish)
    let mut is_effective_line: Vec<bool> = Vec::with_capacity(total_lines);
    is_effective_line.extend(norm_lines.iter().map(|l| !is_blank_or_comment(l)));

    // ----------------------------------
    // 1) Duplicate lines (normalized)
    // ----------------------------------
    let mut seen: HashSet<&str> = HashSet::new();
    let mut dup_line_count = 0usize;
    for (i, l) in norm_lines.iter().enumerate() {
        if !is_effective_line[i] || l.is_empty() { continue; }
        if !seen.insert(l.as_str()) {
            dup_line_count += 1;
        }
    }
    if dup_line_count > 0 {
        let pen = 3.min(dup_line_count as i32); // small, scaled penalty
        penalties += pen;
        notes.push(Note {
            line: 0,
            kind: "duplication_lines".into(),
            severity: "info".into(),
            msg: format!("Duplicate (normalized) lines detected: {}", dup_line_count),
        });
    }

    // --------------------------------------------------------
    // 2) Duplicate blocks (3-line shingles) over effective lines
    // --------------------------------------------------------
    let mut shingle_counts: HashMap<String, usize> = HashMap::new();
    let mut shingle_dups = 0usize;
    let mut window: Vec<&str> = Vec::with_capacity(3);
    let mut positions: Vec<usize> = Vec::with_capacity(3);

    for (i, l) in norm_lines.iter().enumerate() {
        if !is_effective_line[i] || l.is_empty() { continue; }
        window.push(l);
        positions.push(i + 1);
        if window.len() == 3 {
            let key = window.join("\u{241F}"); // unit separator-like delimiter
            let c = shingle_counts.entry(key).or_insert(0);
            *c += 1;
            if *c == 2 {
                // only count when we first discover it's duplicated
                shingle_dups += 1;
            }
            // slide
            window.remove(0);
            positions.remove(0);
        }
    }

    if shingle_dups > 0 {
        let penalty = (shingle_dups as i32).min(6); // cap penalty
        penalties += penalty;
        notes.push(Note {
            line: 0,
            kind: "duplication_blocks".into(),
            severity: "warn".into(),
            msg: format!("Repeated 3-line blocks detected: {}", shingle_dups),
        });
    }

    // --------------------------------
    // 3) Function-like declarations
    // --------------------------------
    let func_like_markers = [
        "fn ", "function ", "function(", "def ", "class ", "struct ", "enum ", "trait ", "interface ",
    ];
    let mut func_like_count: usize = 0;
    for m in func_like_markers {
        func_like_count += code.matches(m).count();
    }
    if func_like_count > 50 {
        penalties += 5;
        notes.push(Note {
            line: 0,
            kind: "function_density".into(),
            severity: "info".into(),
            msg: format!("High count of function-like declarations ({})", func_like_count),
        });
    } else if func_like_count > 25 {
        penalties += 3;
        notes.push(Note {
            line: 0,
            kind: "function_density".into(),
            severity: "info".into(),
            msg: format!("Elevated function-like count ({})", func_like_count),
        });
    }

    // --------------------------------
    // 4) Nesting depth (brace-based)
    // --------------------------------
    let mut cur_brace = 0i32;
    let mut cur_paren = 0i32;
    let mut cur_brack = 0i32;
    let mut max_brace = 0i32;
    let mut max_paren = 0i32;
    let mut max_brack = 0i32;

    // Also track indentation depth (tabs count as 4 visually here)
    let mut max_indent_levels = 0usize;

    // Simple long-if/else chain detector
    let mut current_if_chain = 0usize;
    let mut max_if_chain = 0usize;

    // Detect long functions & param counts by scanning “signature” lines
    let mut long_functions_found = 0usize;
    let mut many_params_found = 0usize;

    // Naive block tracking: when we see a function-like line, start measuring until matching '}' or dedent/blank
    let mut measuring_func = false;
    let mut func_start_line = 0usize;
    let mut func_lines = 0usize;
    let mut brace_at_func_start = 0i32;
    let mut indent_at_func_start: Option<usize> = None;

    for (i, raw) in raw_lines.iter().enumerate() {
        let line_no = i + 1;
        let line = raw.trim_end();

        // indentation depth (spaces=1, tabs=4)
        let (indent_cols, indent_levels) = leading_indent_columns_levels(line);
        if indent_levels > max_indent_levels {
            max_indent_levels = indent_levels;
        }

        // if/else chain
        let lt = line.trim_start();
        let is_if = starts_with_any(lt, &["if ", "if(", "elif ", "else if"]);
        let is_else = starts_with_any(lt, &["else:", "else ", "else{", "else{", "else{", "else{"]);
        if is_if || is_else {
            current_if_chain += 1;
            if current_if_chain > max_if_chain { max_if_chain = current_if_chain; }
        } else if !lt.is_empty() {
            // break the chain on other non-empty lines
            current_if_chain = 0;
        }

        // Count params on likely signature lines (only once per signature line)
        if looks_like_signature(lt) {
            if let Some(n) = count_params_in_parens(lt) {
                if n >= 7 {
                    many_params_found += 1;
                    penalties += 1;
                    notes.push(Note {
                        line: line_no,
                        kind: "many_parameters".into(),
                        severity: "info".into(),
                        msg: format!("Function-like declaration has many parameters ({})", n),
                    });
                }
            }
        }

        // Long function measurement (very heuristic)
        if !measuring_func && is_function_like_start(lt) {
            measuring_func = true;
            func_start_line = line_no;
            func_lines = 0;
            brace_at_func_start = cur_brace;
            indent_at_func_start = Some(indent_levels);
        }

        if measuring_func {
            func_lines += 1;
            // Stop if brace-based: we returned to the starting brace level and line isn’t the start
            let brace_closed = cur_brace <= brace_at_func_start && line_no > func_start_line;
            // Or indent-based: dedented back to (or above) the start indent (for Python-like)
            let indent_closed = if let Some(start_il) = indent_at_func_start {
                indent_levels <= start_il && line_no > func_start_line && !line.trim().is_empty()
            } else { false };

            // Or blank line after some body lines (fallback heuristic)
            let blank_break = func_lines > 3 && line.trim().is_empty();

            if brace_closed || indent_closed || blank_break {
                if func_lines >= 120 {
                    long_functions_found += 1;
                    penalties += 2;
                    notes.push(Note {
                        line: func_start_line,
                        kind: "long_function".into(),
                        severity: "info".into(),
                        msg: format!("Very long function-like block (~{} lines)", func_lines),
                    });
                } else if func_lines >= 80 {
                    long_functions_found += 1;
                    penalties += 1;
                    notes.push(Note {
                        line: func_start_line,
                        kind: "long_function".into(),
                        severity: "info".into(),
                        msg: format!("Long function-like block (~{} lines)", func_lines),
                    });
                }
                measuring_func = false;
            }
        }

        // Update brace/paren/bracket depth across the characters
        for ch in line.chars() {
            match ch {
                '{' => { cur_brace += 1; if cur_brace > max_brace { max_brace = cur_brace; } }
                '}' => { cur_brace -= 1; }
                '(' => { cur_paren += 1; if cur_paren > max_paren { max_paren = cur_paren; } }
                ')' => { cur_paren -= 1; }
                '[' => { cur_brack += 1; if cur_brack > max_brack { max_brack = cur_brack; } }
                ']' => { cur_brack -= 1; }
                _ => {}
            }
        }
    }

    // Penalize excessive nesting (brace-based)
    if max_brace > 6 {
        penalties += 5;
        notes.push(Note {
            line: 0,
            kind: "nesting_depth".into(),
            severity: "warn".into(),
            msg: format!("Excessive block nesting depth ({{}}) = {}", max_brace),
        });
    } else if max_brace > 4 {
        penalties += 3;
        notes.push(Note {
            line: 0,
            kind: "nesting_depth".into(),
            severity: "info".into(),
            msg: format!("High block nesting depth ({{}}) = {}", max_brace),
        });
    }

    // Penalize excessive indentation depth (indent-based)
    if max_indent_levels >= 8 {
        penalties += 2;
        notes.push(Note {
            line: 0,
            kind: "indent_nesting".into(),
            severity: "info".into(),
            msg: format!("High indentation nesting depth ≈ {}", max_indent_levels),
        });
    }

    // Long if/else chains
    if max_if_chain >= 10 {
        penalties += 3;
        notes.push(Note {
            line: 0,
            kind: "long_if_else_chain".into(),
            severity: "warn".into(),
            msg: format!("Very long if/else chain ({} segments)", max_if_chain),
        });
    } else if max_if_chain >= 6 {
        penalties += 1;
        notes.push(Note {
            line: 0,
            kind: "long_if_else_chain".into(),
            severity: "info".into(),
            msg: format!("Long if/else chain ({} segments)", max_if_chain),
        });
    }

    // File size hint (very gentle)
    if total_lines > 2000 {
        penalties += 2;
        notes.push(Note {
            line: 0,
            kind: "file_size".into(),
            severity: "info".into(),
            msg: format!("Large file (~{} lines); consider splitting modules", total_lines),
        });
    }

    // normalize score against total line count
    let total = total_lines.max(1) as f32;
    let score_f = 100.0 - (penalties as f32 * 100.0 / total);
    score_f.clamp(0.0, 100.0) as u32
}

/* ----------------- helpers ----------------- */

fn normalize_line_for_dups(s: &str) -> String {
    // Strip trailing spaces/tabs, fold internal runs of spaces/tabs into a single space.
    // Lowercase to be forgiving across languages (optional; helps catch near-dups).
    let t = s.trim_end().chars().map(|c| if c == '\t' { ' ' } else { c }).collect::<String>();
    let mut out = String::with_capacity(t.len());
    let mut prev_space = false;
    for ch in t.chars() {
        let is_space = ch.is_whitespace();
        if is_space {
            if !prev_space { out.push(' '); }
        } else {
            out.push(ch);
        }
        prev_space = is_space;
    }
    out.trim().to_ascii_lowercase()
}

fn is_blank_or_comment(s: &str) -> bool {
    let t = s.trim_start();
    t.is_empty()
        || t.starts_with("//")
        || t.starts_with('#')
        || t.starts_with("/*")
        || t.starts_with("* ")
        || t.starts_with("-- ")   // SQL
        || t.starts_with("<!--")  // HTML
        || t.starts_with(";")     // .ini/.lisp
}

fn starts_with_any(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| s.starts_with(p))
}

fn looks_like_signature(s: &str) -> bool {
    // Heuristic: a function-like keyword + '(' before end-of-line
    starts_with_any(s, &["fn ", "function ", "def ", "class ", "struct ", "enum ", "trait ", "interface "])
        || (s.contains('(') && (s.contains(")") || s.contains('{')))
}

fn count_params_in_parens(s: &str) -> Option<usize> {
    // Take the first balanced (...) span (if any) on the line and count commas (+1 if non-empty).
    let bytes: Vec<char> = s.chars().collect();
    let mut depth = 0i32;
    let mut start = None;
    for (i, &c) in bytes.iter().enumerate() {
        if c == '(' {
            depth += 1;
            if start.is_none() { start = Some(i + 1); }
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                let st = start?;
                let inner: String = bytes[st..i].iter().collect();
                let trimmed = inner.trim();
                if trimmed.is_empty() { return Some(0); }
                // Count commas not inside nested parentheses (simple heuristic)
                let mut par = 0i32;
                let mut commas = 0usize;
                for ch in trimmed.chars() {
                    match ch {
                        '(' => par += 1,
                        ')' => par -= 1,
                        ',' if par == 0 => commas += 1,
                        _ => {}
                    }
                }
                return Some(commas + 1);
            }
        }
    }
    None
}

fn is_function_like_start(s: &str) -> bool {
    starts_with_any(s, &["fn ", "function ", "def ", "class ", "struct ", "enum ", "trait ", "interface "])
}

fn leading_indent_columns_levels(s: &str) -> (usize, usize) {
    // Returns (columns, levels) where levels ~ columns/4 (rough heuristic)
    let mut cols = 0usize;
    for ch in s.chars() {
        match ch {
            ' ' => cols += 1,
            '\t' => cols += 4,
            _ => break,
        }
    }
    let levels = cols / 4;
    (cols, levels)
}