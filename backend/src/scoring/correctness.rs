use crate::models::Note;
use super::language::Language;

pub fn score_correctness(code: &str, lang: &Language, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;

    // Structure/bracket checks per language (string/comment-aware)
    match lang {
        Language::Javascript | Language::Css => {
            score += check_brackets(code, BracketMode::JsCss, notes);
        }
        Language::Python => {
            score += check_brackets(code, BracketMode::Python, notes);
        }
        Language::Html => {
            // (HTML uses tags; handled below)
        }
        _ => { /* no-op for other languages for now */ }
    }

    // Language-specific semantic heuristics
    match lang {
        Language::Html => {
            score += check_html_structure(code, notes);
        }
        Language::Css => {
            score += check_css_rules(code, notes);
        }
        Language::Javascript => {
            score += check_js_semicolons_better(code, notes);
            score += check_js_comparisons_and_assignments(code, notes);
        }
        Language::Python => {
            score += check_python_blocks_and_common_pitfalls(code, notes);
        }
        _ => {}
    }

    score.clamp(0, 100) as u32
}

/* -------------------------
   Helpers & checks (no deps)
   ------------------------- */

fn push_note(notes: &mut Vec<Note>, line: usize, kind: &str, severity: &str, msg: impl Into<String>) {
    notes.push(Note {
        line,
        kind: kind.into(),
        severity: severity.into(),
        msg: msg.into(),
    });
}

/* ===== Bracket scanning (string/comment-aware) ===== */

#[derive(Copy, Clone)]
struct Pos { line: usize, col: usize }

#[derive(Copy, Clone)]
struct Br { ch: char, pos: Pos }

#[derive(Copy, Clone, Eq, PartialEq)]
enum BracketMode {
    JsCss,
    Python,
}

fn check_brackets(code: &str, mode: BracketMode, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;
    let mut stack: Vec<Br> = Vec::new();

    let chars: Vec<char> = code.chars().collect();

    let mut line = 1usize;
    let mut col = 0usize;
    let mut i = 0usize;

    // String state for both JS/CSS and Python
    #[derive(Copy, Clone)]
    enum StrState {
        // JS/CSS: quote (' or ") or backtick (`).
        Js(char),
        // Python: quote (' or ") with len=1 or 3; raw strings ignore escapes.
        Py { quote: char, triple: bool, raw: bool },
    }
    let mut in_str: Option<StrState> = None;

    // Comment state (JS/CSS only)
    let mut in_ml_comment = false;

    // For JS/CSS comment open detection like "/*" and "//"
    let mut last_char: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];
        col += 1;

        // Newline handling
        if c == '\n' {
            line += 1;
            col = 0;
            last_char = Some(c);
            i += 1;
            continue;
        }

        // ===== If currently inside a string, handle and continue =====
        if let Some(st) = in_str.take() {
            match st {
                StrState::Js(q) => {
                    if c == q {
                        // handle escapes for ' " and `
                        let mut esc = false;
                        let mut j = i;
                        while j > 0 && chars[j - 1] == '\\' {
                            esc = !esc;
                            j -= 1;
                        }
                        if esc {
                            // still in the string
                            in_str = Some(StrState::Js(q));
                        } // else: closed, leave None
                    } else {
                        // stay in the string
                        in_str = Some(StrState::Js(q));
                    }
                    last_char = Some(c);
                    i += 1;
                    continue;
                }
                StrState::Py { quote, triple, raw } => {
                    if triple {
                        // Close on three consecutive quotes
                        if c == quote && i + 2 < chars.len()
                            && chars[i + 1] == quote && chars[i + 2] == quote
                        {
                            // closed: consume the other two quotes
                            i += 3;
                            col += 2;
                            last_char = Some(quote);
                            continue;
                        } else {
                            // still inside triple-quoted string
                            in_str = Some(StrState::Py { quote, triple: true, raw });
                            last_char = Some(c);
                            i += 1;
                            continue;
                        }
                    } else {
                        if c == quote {
                            if raw {
                                // raw single-quoted string closes immediately
                                // leave in_str = None
                            } else {
                                // check escaping backslashes
                                let mut esc = false;
                                let mut j = i;
                                while j > 0 && chars[j - 1] == '\\' {
                                    esc = !esc;
                                    j -= 1;
                                }
                                if esc {
                                    // escaped quote; stay in the string
                                    in_str = Some(StrState::Py { quote, triple: false, raw });
                                }
                            }
                            last_char = Some(c);
                            i += 1;
                            continue;
                        } else {
                            // not the closing quote; remain in the string
                            in_str = Some(StrState::Py { quote, triple: false, raw });
                            last_char = Some(c);
                            i += 1;
                            continue;
                        }
                    }
                }
            }
        }

        // ===== Comments / string entry by language =====
        match mode {
            BracketMode::JsCss => {
                // Inside multi-line comment?
                if in_ml_comment {
                    if last_char == Some('*') && c == '/' {
                        in_ml_comment = false;
                    }
                    last_char = Some(c);
                    i += 1;
                    continue;
                }

                // Enter single-line comment: //
                if last_char == Some('/') && c == '/' {
                    // skip until end of line
                    while i < chars.len() && chars[i] != '\n' { i += 1; }
                    last_char = Some('/');
                    continue;
                }
                // Enter multi-line comment: /*
                if last_char == Some('/') && c == '*' {
                    in_ml_comment = true;
                    last_char = Some('*');
                    i += 1;
                    continue;
                }
                // Enter string?
                if c == '\'' || c == '"' || c == '`' {
                    in_str = Some(StrState::Js(c));
                    last_char = Some(c);
                    i += 1;
                    continue;
                }
            }
            BracketMode::Python => {
                // Single-line comment: #
                if c == '#' {
                    // skip until end of line
                    while i < chars.len() && chars[i] != '\n' { i += 1; }
                    last_char = Some('#');
                    continue;
                }
                // Enter string? (single, double, or triple; consider prefixes r/f/b/u)
                if c == '\'' || c == '"' {
                    // look back for prefixes (r, f, b, u, combinations like fr/rf)
                    let mut j = i;
                    while j > 0 && chars[j - 1].is_ascii_alphabetic() { j -= 1; }
                    let prefix: String = chars[j..i].iter().collect();
                    let p = prefix.to_ascii_lowercase();
                    let raw = p.contains('r');
                    let triple = i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c;

                    if triple {
                        in_str = Some(StrState::Py { quote: c, triple: true, raw });
                        i += 3;
                        col += 2;
                        last_char = Some(c);
                        continue;
                    } else {
                        in_str = Some(StrState::Py { quote: c, triple: false, raw });
                        i += 1;
                        last_char = Some(c);
                        continue;
                    }
                }
            }
        }

        // ===== Brackets (only when not in string/comment) =====
        let pos = Pos { line, col };
        match c {
            '{' | '(' | '[' => stack.push(Br { ch: c, pos }),
            '}' | ')' | ']' => {
                if let Some(top) = stack.pop() {
                    let ok = matches!((top.ch, c), ('{', '}') | ('(', ')') | ('[', ']'));
                    if !ok {
                        push_note(
                            notes,
                            pos.line,
                            "bracket_mismatch",
                            "warn",
                            format!(
                                "Found '{}' but last open was '{}' (opened at line {}, col {})",
                                c, top.ch, top.pos.line, top.pos.col
                            ),
                        );
                        delta -= 6;
                    }
                } else {
                    push_note(
                        notes,
                        pos.line,
                        "stray_closing_bracket",
                        "warn",
                        format!("Stray closing '{}' at col {}", c, pos.col),
                    );
                    delta -= 6;
                }
            }
            _ => {}
        }

        last_char = Some(c);
        i += 1;
    }

    // Unclosed brackets
    for br in stack {
        push_note(
            notes,
            br.pos.line,
            "unclosed_bracket",
            "warn",
            format!("Unclosed '{}' opened at line {}, col {}", br.ch, br.pos.line, br.pos.col),
        );
        delta -= 6;
    }

    delta
}

/* ===== HTML checks: tag stack + common pitfalls ===== */

fn check_html_structure(code: &str, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;

    // Void elements that do not require closing tags
    const VOID: [&str; 15] = [
        "area","base","br","col","embed","hr","img","input","link","meta","param","source","track","wbr","basefont"
    ];

    #[derive(Clone)]
    struct TagPos { name: String, line: usize, col: usize }

    let mut stack: Vec<TagPos> = Vec::new();
    let mut line = 1usize;
    let mut col = 0usize;

    let chars: Vec<char> = code.chars().collect();
    let mut i = 0usize;

    // helpers
    let is_name_char = |c: char| c.is_alphanumeric() || c == '-' || c == ':'; // namespaced tags possible

    use std::collections::HashMap;
    let mut seen_ids: HashMap<String, (usize, usize)> = HashMap::new();

    while i < chars.len() {
        let c = chars[i];
        col += 1;

        if c == '\n' { line += 1; col = 0; i += 1; continue; }

        if c == '<' {
            // Comments
            if i + 3 < chars.len() && chars[i+1] == '!' && chars[i+2] == '-' && chars[i+3] == '-' {
                i += 4; col += 3;
                // consume until -->
                while i + 2 < chars.len() {
                    if chars[i] == '\n' { line += 1; col = 0; }
                    if chars[i] == '-' && chars[i+1] == '-' && chars[i+2] == '>' {
                        i += 3; col += 3;
                        break;
                    }
                    i += 1; col += 1;
                }
                continue;
            }
            // Doctype or processing instructions
            if i + 1 < chars.len() && (chars[i+1] == '!' || chars[i+1] == '?') {
                i += 2; col += 1;
                while i < chars.len() && chars[i] != '>' {
                    if chars[i] == '\n' { line += 1; col = 0; }
                    i += 1; col += 1;
                }
                i += 1; col += 1;
                continue;
            }

            let tag_line = line;
            let tag_col = col;

            let mut j = i + 1;
            let mut closing = false;
            if j < chars.len() && chars[j] == '/' {
                closing = true;
                j += 1;
            }

            // Read name
            let mut name = String::new();
            while j < chars.len() && is_name_char(chars[j]) {
                name.push(chars[j]);
                j += 1;
            }
            if name.is_empty() {
                i += 1; // move forward
                continue;
            }

            // Parse attributes (very light)
            let mut attr_buf = String::new();
            let mut in_quote: Option<char> = None;
            let mut self_closing = false;

            while j < chars.len() {
                let ch = chars[j];
                if in_quote.is_some() {
                    if Some(ch) == in_quote {
                        in_quote = None;
                    }
                    attr_buf.push(ch);
                    j += 1;
                    continue;
                }
                if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                    attr_buf.push(ch);
                    j += 1;
                    continue;
                }
                if ch == '>' {
                    j += 1;
                    break;
                }
                if ch == '/' && j + 1 < chars.len() && chars[j+1] == '>' {
                    self_closing = true;
                    j += 2;
                    break;
                }
                attr_buf.push(ch);
                j += 1;
            }

            let lname = name.to_ascii_lowercase();

            if closing {
                if let Some(top) = stack.pop() {
                    if top.name != lname {
                        push_note(notes, tag_line, "html_mismatched_tag", "warn",
                            format!("Expected </{}> but found </{}> (opened at line {}, col {})",
                                    top.name, lname, top.line, top.col));
                        delta -= 8;
                    }
                } else {
                    push_note(notes, tag_line, "html_stray_closing_tag", "warn",
                        format!("Stray closing </{}>", lname));
                    delta -= 6;
                }
            } else {
                let is_void = VOID.iter().any(|&v| v == lname) || self_closing;
                if !is_void {
                    stack.push(TagPos { name: lname.clone(), line: tag_line, col: tag_col });
                }

                // <img> missing alt
                if lname == "img" && !attr_buf.to_ascii_lowercase().contains("alt=") {
                    push_note(notes, tag_line, "html_img_missing_alt", "info",
                        "Image tag missing alt attribute");
                    delta -= 2;
                }

                // duplicate id=""
                if let Some(id_val) = extract_id_attr(&attr_buf) {
                    if let Some((first_line, first_col)) = seen_ids.get(&id_val).copied() {
                        push_note(notes, tag_line, "html_duplicate_id", "warn",
                            format!("Duplicate id=\"{}\" (first seen at line {}, col {})",
                                    id_val, first_line, first_col));
                        delta -= 5;
                    } else {
                        seen_ids.insert(id_val, (tag_line, tag_col));
                    }
                }
            }

            i = j;
            continue;
        }

        i += 1;
    }

    for t in stack {
        push_note(notes, t.line, "html_unclosed_tag", "warn",
            format!("Unclosed <{}> (opened at line {}, col {})", t.name, t.line, t.col));
        delta -= 8;
    }

    delta
}

fn extract_id_attr(attrs: &str) -> Option<String> {
    let s = attrs;
    let mut i = 0usize;
    let b: Vec<char> = s.chars().collect();
    while i < b.len() {
        while i < b.len() && b[i].is_whitespace() { i += 1; }
        let start = i;
        while i < b.len() && (b[i].is_alphanumeric() || b[i] == '-' || b[i] == ':' || b[i] == '_') { i += 1; }
        let name: String = b[start..i].iter().collect();
        while i < b.len() && b[i].is_whitespace() { i += 1; }
        if i < b.len() && b[i] == '=' {
            i += 1;
            while i < b.len() && b[i].is_whitespace() { i += 1; }
            if i < b.len() && (b[i] == '"' || b[i] == '\'') {
                let quote = b[i];
                i += 1;
                let val_start = i;
                while i < b.len() && b[i] != quote { i += 1; }
                let val: String = b[val_start..i.min(b.len())].iter().collect();
                if i < b.len() { i += 1; }
                if name.eq_ignore_ascii_case("id") {
                    return Some(val);
                }
            } else {
                let val_start = i;
                while i < b.len() && !b[i].is_whitespace() { i += 1; }
                let val: String = b[val_start..i].iter().collect();
                if name.eq_ignore_ascii_case("id") {
                    return Some(val);
                }
            }
        }
        i += 1;
    }
    None
}

/* ===== CSS checks ===== */

fn check_css_rules(code: &str, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;

    let trimmed = code.trim();
    if !trimmed.is_empty() && !trimmed.ends_with('}') && !trimmed.ends_with(';') {
        push_note(notes, 0, "css_maybe_incomplete", "info",
            "CSS may be incomplete (does not end with '}' or ';')");
        delta -= 2;
    }

    let mut in_block = false;
    let mut prop_since_open = false;
    for (i, raw_line) in code.lines().enumerate() {
        let line_no = i + 1;
        let line = raw_line.trim();

        if line.starts_with("/*") && line.ends_with("*/") { continue; }
        if line.is_empty() { continue; }

        if line.contains('{') { in_block = true; prop_since_open = false; }
        if in_block {
            if line.contains(':') && !line.ends_with('{') && !line.starts_with('@') {
                prop_since_open = true;
                if !line.ends_with(';') && !line.ends_with('}') {
                    push_note(notes, line_no, "css_property_no_semicolon", "info",
                        "CSS property likely missing trailing ';'");
                    delta -= 1;
                }
            } else if !line.contains('}') && !line.ends_with('{') && !line.starts_with('@') {
                if line.chars().any(|c| c.is_alphabetic()) && !line.starts_with("--") {
                    push_note(notes, line_no, "css_property_no_colon", "warn",
                        "Line inside a block may be a property but has no ':'");
                    delta -= 2;
                }
            }
        }
        if line.contains('}') {
            if !prop_since_open {
                push_note(notes, line_no, "css_empty_rule", "info",
                    "Empty CSS rule (no properties between braces)");
                delta -= 1;
            }
            in_block = false;
        }
    }

    delta
}

/* ===== JavaScript checks ===== */

fn check_js_semicolons_better(code: &str, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;

    let continue_suffixes = [
        "(", "[", "{", ",", ":", "?", "&&", "||", "??", "+", "-", "*", "/", "%", "**", "&",
        "|", "^", ">>", "<<", ">>>", "=", "==", "===", "!=", "!==", "+=", "-=", "*=", "/=",
        "%=", "&&=", "||=", "??=", "&=", "|=", "^=", "<<=", ">>=", ">>>=", "=>",
    ];
    let ok_endings = [';', '{', '}', ','];

    let mut in_ml_comment = false;
    let mut in_str: Option<char> = None;
    let mut backslash = false;

    for (i, raw) in code.lines().enumerate() {
        let chars: Vec<char> = raw.chars().collect();
        let mut j = 0usize;
        let mut in_sl_comment = false;

        while j < chars.len() {
            let c = chars[j];
            if in_sl_comment { break; }
            if let Some(q) = in_str {
                if !backslash && c == q { in_str = None; }
                backslash = (!backslash && c == '\\') && q != '`';
                j += 1;
                continue;
            }
            if in_ml_comment {
                if j + 1 < chars.len() && chars[j] == '*' && chars[j+1] == '/' {
                    in_ml_comment = false;
                    j += 2;
                    continue;
                } else { j += 1; continue; }
            }
            if j + 1 < chars.len() && chars[j] == '/' && chars[j+1] == '/' {
                in_sl_comment = true; break;
            }
            if j + 1 < chars.len() && chars[j] == '/' && chars[j+1] == '*' {
                in_ml_comment = true; j += 2; continue;
            }
            if c == '"' || c == '\'' || c == '`' { in_str = Some(c); }
            j += 1;
        }

        let effective = if in_sl_comment {
            &raw[..raw.find("//").unwrap_or(raw.len())]
        } else {
            raw
        };
        let t = effective.trim_end().trim();
        if t.is_empty() || t.starts_with("/*") || in_ml_comment { continue; }
        if t.starts_with("//") { continue; }

        if ok_endings.iter().any(|&ch| t.ends_with(ch)) { continue; }
        if continue_suffixes.iter().any(|s| t.ends_with(s)) { continue; }

        let starters = ["if", "for", "while", "switch", "try", "catch", "finally", "function", "class", "do", "else"];
        if starters.iter().any(|kw| t.starts_with(kw)) { continue; }

        push_note(notes, i + 1, "js_semicolon", "info", "Potential missing semicolon (heuristic)");
        delta -= 1;
    }

    delta
}

fn check_js_comparisons_and_assignments(code: &str, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;

    for (i, line) in code.lines().enumerate() {
        let t = line.trim();
        if t.contains("==") && !t.contains("===") {
            push_note(notes, i + 1, "js_loose_equality", "info",
                "Consider using '===' instead of '==' to avoid coercion surprises");
            delta -= 1;
        }
        if t.contains("!=") && !t.contains("!==") {
            push_note(notes, i + 1, "js_loose_inequality", "info",
                "Consider using '!==' instead of '!=' to avoid coercion surprises");
            delta -= 1;
        }
    }

    // Assignment inside condition parens
    for (i, line) in code.lines().enumerate() {
        let mut depth = 0i32;
        let b: Vec<char> = line.chars().collect();
        let mut j = 0usize;
        while j < b.len() {
            if b[j] == '(' { depth += 1; }
            if b[j] == ')' { depth -= 1; }
            if depth > 0 && b[j] == '=' {
                let prev = if j > 0 { b[j - 1] } else { '\0' };
                let next = if j + 1 < b.len() { b[j + 1] } else { '\0' };
                if !(prev == '=' || next == '=' || next == '>') {
                    let lt = line.trim_start();
                    if lt.starts_with("if ")
                        || lt.starts_with("if(")
                        || lt.starts_with("while ")
                        || lt.starts_with("while(")
                        || lt.starts_with("for ")
                        || lt.starts_with("for(")
                    {
                        push_note(notes, i + 1, "js_assignment_in_condition", "warn",
                            "Possible accidental assignment '=' inside condition; did you mean '==' or '==='?");
                        delta -= 4;
                        break;
                    }
                }
            }
            j += 1;
        }
    }

    delta
}

/* ===== Python checks ===== */

fn check_python_blocks_and_common_pitfalls(code: &str, notes: &mut Vec<Note>) -> i32 {
    let mut delta = 0i32;

    // 1) Lines that introduce a block must end with ':'
    let starters = [
        "def ", "class ", "if ", "elif ", "else", "for ", "while ", "try", "except", "finally", "with ",
        "match ", "case ",
    ];

    // Track if file mixes tabs/spaces in indentation
    let mut saw_tab_indent = false;
    let mut saw_space_indent = false;

    for (i, raw) in code.lines().enumerate() {
        let line_no = i + 1;

        // Skip empty lines
        if raw.trim().is_empty() { continue; }

        // Detect leading indentation
        let indent_len = raw.chars().take_while(|c| c.is_whitespace()).count();
        let indent: String = raw.chars().take(indent_len).collect();
        if indent.contains('\t') { saw_tab_indent = true; }
        if indent.contains(' ') { saw_space_indent = true; }

        // Strip leading spaces/tabs for logic checks
        let t = raw[indent_len..].trim_end();

        // Skip full-line comments
        if t.starts_with('#') { continue; }

        // Heuristic: if a starter, should end with ':'
        if starters.iter().any(|p| t.starts_with(p)) {
            if !t.ends_with(':') {
                push_note(notes, line_no, "py_missing_colon", "warn",
                    "Python block statement appears to be missing a trailing ':'");
                delta -= 3;
            }
        }

        // 2) '== None' or '!= None' -> prefer 'is None' / 'is not None'
        let lower = t.to_ascii_lowercase();
        if lower.contains("== none") {
            push_note(notes, line_no, "py_none_equality", "info",
                "Use 'is None' instead of '== None'");
            delta -= 1;
        }
        if lower.contains("!= none") {
            push_note(notes, line_no, "py_none_inequality", "info",
                "Use 'is not None' instead of '!= None'");
            delta -= 1;
        }
    }

    if saw_tab_indent && saw_space_indent {
        push_note(notes, 0, "py_mixed_indentation", "warn",
            "File mixes tabs and spaces in indentation; pick one (PEP8 recommends 4 spaces)");
        delta -= 3;
    }

    delta
}