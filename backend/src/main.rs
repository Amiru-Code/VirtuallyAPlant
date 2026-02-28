use axum::{
    body::to_bytes,
    extract::{DefaultBodyLimit, Request, State},
    http::{Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use tower_http::cors::{Any, CorsLayer};

/// --- App state (room for config later)
#[derive(Clone)]
struct AppState {
    max_line_len: usize,
    max_body_bytes: usize,
}

/// --- Public API types (JSON)
#[derive(Debug, Deserialize)]
struct JudgeReq {
    code: String,
    /// Optional explicit language ("html","css","js","python","rust","auto")
    lang: Option<String>,
}

#[derive(Debug, Serialize)]
struct JudgeResp {
    cleanliness: u32,
    correctness: u32,
    structure: u32,
    overall: u32,
    notes: Vec<Note>,
}

#[derive(Debug, Serialize)]
struct Note {
    line: usize,
    kind: String,
    severity: String, // "info" | "warn" | "error"
    msg: String,
}

/// --- Language enum used internally
#[derive(Debug)]
enum Language {
    Html,
    Css,
    Javascript,
    Python,
    Rust,
    Unknown,
}

#[tokio::main]
async fn main() {
    // Dev-friendly CORS; lock down origins in production
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let state = Arc::new(AppState {
        max_line_len: 120,
        max_body_bytes: 1 * 1024 * 1024, // 1 MiB
    });

    // Build router INSIDE main() (avoids `let` at module root errors)
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))                       // quick GET probe
        .route("/judge", post(judge_text))                              // raw text -> JSON
        .route("/judge/json", post(judge_json))                         // JSON -> JSON
        .route("/demo", get(demo_form))                                 // optional: test page
        .layer(cors)
        .layer(DefaultBodyLimit::max(state.max_body_bytes))             // body size limit
        .with_state(state);

    println!("🚀 Rust server running at http://localhost:3000");
    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

/// --- GET /demo (tiny HTML form for manual testing)
async fn demo_form() -> Html<&'static str> {
    Html(
        r#"<!doctype html><meta charset="utf-8" />
        <body style="font-family: system-ui; margin: 2rem;">
          <h1>Judge (POST /judge)</h1>
          <form method="POST" action="/judge">
            <textarea name="code" rows="12" cols="80">// paste or type code here</textarea><br/><br/>
            <button type="submit">Judge (as text)</button>
          </form>
          <p>Tip: This form posts <code>application/x-www-form-urlencoded</code>.
          The server reads the raw body for <code>/judge</code>, so curl or your frontend is best.</p>
        </body>"#,
    )
}

/// --- POST /judge (raw text -> JSON), matches your current frontend
/// Your frontend does: headers: { "Content-Type": "text/plain" }, body: file text. ✔
/// We'll accept any content-type here and treat the entire body as code.
async fn judge_text(State(app): State<Arc<AppState>>, req: Request) -> impl IntoResponse {
    match to_bytes(req.into_body(), app.max_body_bytes).await {
        Ok(bytes) => {
            let code = match String::from_utf8(bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => return (StatusCode::BAD_REQUEST, "Body is not valid UTF-8").into_response(),
            };
            let lang = detect_language(&code);
            let (resp, status) = compute_all(&code, lang, app.max_line_len);
            (status, Json(resp)).into_response()
        }
        Err(_) => (StatusCode::PAYLOAD_TOO_LARGE, "Body too large").into_response(),
    }
}

/// --- POST /judge/json (JSON -> JSON) for future use
async fn judge_json(
    State(app): State<Arc<AppState>>,
    Json(req): Json<JudgeReq>,
) -> impl IntoResponse {
    let lang = match req.lang.as_deref() {
        Some("html") => Language::Html,
        Some("css") => Language::Css,
        Some("js" | "javascript") => Language::Javascript,
        Some("python" | "py") => Language::Python,
        Some("rust" | "rs") => Language::Rust,
        _ => detect_language(&req.code),
    };
    let (resp, status) = compute_all(&req.code, lang, app.max_line_len);
    (status, Json(resp))
}

/// --- Single function to run all checks, aggregate results & notes
fn compute_all(code: &str, lang: Language, max_line_len: usize) -> (JudgeResp, StatusCode) {
    let mut notes = Vec::<Note>::new();

    // Style/Cleanliness notes
    let c1 = score_cleanliness(code, max_line_len, &mut notes);

    // Correctness notes (still light heuristics; you can deepen later)
    let c2 = score_correctness(code, &lang, &mut notes);

    // Structure notes
    let c3 = score_structure(code, &mut notes);

    let overall = ((c1 + c2 + c3) / 3).clamp(0, 100);

    (
        JudgeResp {
            cleanliness: c1,
            correctness: c2,
            structure: c3,
            overall,
            notes,
        },
        StatusCode::OK,
    )
}

/// --- Language detection (lightweight heuristic)
fn detect_language(code: &str) -> Language {
    let lower = code.to_lowercase();
    if lower.contains("<html") || lower.contains("<div") {
        return Language::Html;
    }
    // extremely naive CSS sniff: many rules with { } and :
    if lower.contains('{') && lower.contains('}') && lower.contains(':') && lower.contains(';') {
        return Language::Css;
    }
    if lower.contains("function") || lower.contains("let ") || lower.contains("const ") {
        return Language::Javascript;
    }
    if lower.contains("def ") || lower.contains("import ") {
        return Language::Python;
    }
    if lower.contains("fn ") || lower.contains("pub ") || lower.contains("mod ") {
        return Language::Rust;
    }
    Language::Unknown
}

/// --- Cleanliness score with line-level notes
fn score_cleanliness(code: &str, max_len: usize, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;
    for (i, line) in code.lines().enumerate() {
        let line_no = i + 1;
        if line.ends_with(' ') || line.ends_with('\t') {
            score -= 1;
            notes.push(Note {
                line: line_no,
                kind: "trailing_whitespace".into(),
                severity: "info".into(),
                msg: "Trailing whitespace".into(),
            });
        }
        if line.chars().count() > max_len {
            score -= 1;
            notes.push(Note {
                line: line_no,
                kind: "line_length".into(),
                severity: "info".into(),
                msg: format!("Line exceeds {} characters", max_len),
            });
        }
    }
    score.clamp(0, 100) as u32
}

/// --- Correctness heuristics (very lightweight)
fn score_correctness(code: &str, lang: &Language, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;

    // braces balance (generic)
    let open_braces = code.matches('{').count();
    let close_braces = code.matches('}').count();
    if open_braces != close_braces {
        score -= 10;
        notes.push(Note {
            line: 0,
            kind: "brace_balance".into(),
            severity: "warn".into(),
            msg: format!("Unbalanced braces: open {} vs close {}", open_braces, close_braces),
        });
    }

    match lang {
        Language::Html => {
            let opens = code.matches("<div").count();
            let closes = code.matches("</div>").count();
            if opens != closes {
                score -= 8;
                notes.push(Note {
                    line: 0,
                    kind: "html_div_balance".into(),
                    severity: "warn".into(),
                    msg: format!("<div> count {} != </div> count {}", opens, closes),
                });
            }
        }
        Language::Css => {
            if !code.trim().is_empty() && !code.trim().ends_with('}') {
                score -= 5;
                notes.push(Note {
                    line: 0,
                    kind: "css_block_end".into(),
                    severity: "info".into(),
                    msg: "CSS seems not to end with a '}'".into(),
                });
            }
        }
        Language::Javascript => {
            // quick-and-dirty semicolon heuristic
            for (i, line) in code.lines().enumerate() {
                let t = line.trim_end();
                if t.is_empty() || t.starts_with("//") || t.starts_with("/*") {
                    continue;
                }
                if !(t.ends_with(';') || t.ends_with('{') || t.ends_with('}') || t.ends_with(',') || t.ends_with(':')) {
                    score -= 1;
                    notes.push(Note {
                        line: i + 1,
                        kind: "js_semicolon".into(),
                        severity: "info".into(),
                        msg: "Potential missing semicolon (heuristic)".into(),
                    });
                }
            }
        }
        _ => {}
    }

    score.clamp(0, 100) as u32
}

/// --- Structure heuristics (dup lines + function count)
fn score_structure(code: &str, notes: &mut Vec<Note>) -> u32 {
    let mut score: i32 = 100;

    // very rough duplication check (exact duplicate lines)
    let lines: Vec<&str> = code.lines().collect();
    let unique: HashSet<&str> = lines.iter().copied().collect();
    if unique.len() < lines.len() {
        score -= 5;
        notes.push(Note {
            line: 0,
            kind: "duplication".into(),
            severity: "info".into(),
            msg: "Duplicate lines detected".into(),
        });
    }

    // naive function density penalty
    let func_like = ["function", "fn "];
    let func_count = func_like.iter().map(|k| code.matches(k).count()).sum::<usize>();
    if func_count > 25 {
        score -= 5;
        notes.push(Note {
            line: 0,
            kind: "function_density".into(),
            severity: "info".into(),
            msg: format!("High function-like count ({})", func_count),
        });
    }

    score.clamp(0, 100) as u32
}