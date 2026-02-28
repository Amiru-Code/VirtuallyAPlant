#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Html,
    Css,
    Javascript,
    Python,
    Rust,
    Unknown,
}

/// Heuristic language detection used by the judge.
///
/// The intent is not to build a full parser but to make a best‑effort guess so
/// that other scoring rules can apply language‑specific heuristics.  We
/// perform a single lowercase transformation (ASCII only) and then test for a
/// handful of distinctive tokens.  The order is important: for example HTML is
/// checked before CSS because `"<div> { ... }"` would otherwise look like
/// CSS.
///
/// The function is intentionally conservative; if no pattern matches it returns
/// `Language::Unknown` rather than guessing incorrectly.
pub fn detect_language(code: &str) -> Language {
    let text = code.to_ascii_lowercase();

    if text.contains("<html") || text.contains("<div") {
        return Language::Html;
    }

    if text.contains('{') && text.contains('}') && text.contains(':') && text.contains(';') {
        return Language::Css;
    }

    if text.contains("function") || text.contains("let ") || text.contains("const ") {
        return Language::Javascript;
    }

    if text.contains("def ") || text.contains("import ") {
        return Language::Python;
    }

    if text.contains("fn ") || text.contains("pub ") || text.contains("mod ") {
        return Language::Rust;
    }

    Language::Unknown
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_html() {
        assert_eq!(detect_language("<HTML><Div>"), Language::Html);
    }

    #[test]
    fn detect_css() {
        assert_eq!(detect_language("body { color: red; }"), Language::Css);
    }

    #[test]
    fn detect_js() {
        assert_eq!(detect_language("function foo() { let x = 1; }"), Language::Javascript);
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect_language("def foo():\n    import os"), Language::Python);
    }

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language("pub fn main() {}"), Language::Rust);
    }

    #[test]
    fn unknown_code() {
        assert_eq!(detect_language("just some plain text"), Language::Unknown);
    }
}

