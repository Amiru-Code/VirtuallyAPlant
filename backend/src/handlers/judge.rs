use axum::{body::to_bytes, extract::{Request, State}, http::StatusCode, response::IntoResponse, Json};
use std::{sync::Arc, fs::OpenOptions, io::Write, path::Path};

use crate::{state::AppState, models::{JudgeReq, JudgeResp, Note}, scoring::{compute_all, language::detect_language}};

/// Append each note to a JSON-lines file for later inspection.
///
/// The file is called `notes.jsonl` in the current working directory; each
/// entry is the serialization of a single `Note` followed by a newline.  If the
/// file cannot be opened or written to, an error is returned but the request
/// still succeeds.
fn persist_notes(notes: &[Note]) -> std::io::Result<()> {
    if notes.is_empty() {
        return Ok(());
    }
    let path = Path::new("notes.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for note in notes {
        let line = serde_json::to_string(note).unwrap_or_else(|_| "{}".into());
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

pub async fn judge_text(
    State(app): State<Arc<AppState>>,
    req: Request,
) -> impl IntoResponse {
    match to_bytes(req.into_body(), app.max_body_bytes).await {
        Ok(bytes) => {
            let code = match String::from_utf8(bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => return (StatusCode::BAD_REQUEST, "Body is not valid UTF-8").into_response(),
            };
            let lang = detect_language(&code);
            let (resp, status) = compute_all(&code, lang, app.max_line_len);
            // persist notes to disk (append as newline-delimited JSON)
            if let Err(e) = persist_notes(&resp.notes) {
                eprintln!("failed to write notes: {}", e);
            }
            (status, Json(resp)).into_response()
        }
        Err(_) => (StatusCode::PAYLOAD_TOO_LARGE, "Body too large").into_response(),
    }
}

pub async fn judge_json(
    State(app): State<Arc<AppState>>,
    Json(req): Json<JudgeReq>,
) -> impl IntoResponse {
    use crate::scoring::language::Language;
    let lang = match req.lang.as_deref() {
        Some("html") => Language::Html,
        Some("css") => Language::Css,
        Some("js") | Some("javascript") => Language::Javascript,
        Some("python") | Some("py") => Language::Python,
        Some("rust") | Some("rs") => Language::Rust,
        _ => detect_language(&req.code),
    };
    let (resp, status) = compute_all(&req.code, lang, app.max_line_len);
    if let Err(e) = persist_notes(&resp.notes) {
        eprintln!("failed to write notes: {}", e);
    }
    (status, Json(resp))
}
