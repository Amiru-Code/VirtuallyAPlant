#[derive(Debug, Clone, Copy)]
pub enum Language { Html, Css, Javascript, Python, Rust, Unknown }


\\ detects the language being used and uses this information to score the efficiency of the program
pub fn detect_language(code: &str) -> Language {
    let lower = code.to_lowercase();
    if lower.contains("<html") || lower.contains("<div")     { return Language::Html; }
    if lower.contains('{') && lower.contains('}')
        && lower.contains(':') && lower.contains(';')        { return Language::Css; }
    if lower.contains("function") || lower.contains("let ")
        || lower.contains("const ")                         { return Language::Javascript; }
    if lower.contains("def ") || lower.contains("import ")  { return Language::Python; }
    if lower.contains("fn ") || lower.contains("pub ")
        || lower.contains("mod ")                           { return Language::Rust; }
    Language::Unknown
}