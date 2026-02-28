use axum::{
    body::to_bytes,
    extract::Request,
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::Serialize;
use std::collections::HashSet;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug)]
enum Language {
    Html,
    Css,
    Javascript,
    Python,
    Rust,
    Unknown,
}

fn detect_language(code: &str) -> Language {
    let lower = code.to_lowercase();

    if lower.contains("<html") || lower.contains("<div") {
        return Language::Html;
    }
    if lower.contains("{") && lower.contains("}") && lower.contains(":") && lower.contains(";") {
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

fn score_cleanliness(code: &str) -> u32 {
    let mut score = 100;

    for line in code.lines() {
        if line.ends_with(' ') {
            score -= 1;
        }
        if line.len() > 120 {
            score -= 1;
        }
    }

    score.clamp(0, 100)
}

fn score_correctness(code: &str, lang: &Language) -> u32 {
    let mut score = 100;

    let open_braces = code.matches('{').count();
    let close_braces = code.matches('}').count();
    if open_braces != close_braces {
        score -= 20;
    }

    match lang {
        Language::Html => {
            if code.matches("<div").count() != code.matches("</div>").count() {
                score -= 20;
            }
        }
        Language::Css => {
            if !code.trim().ends_with('}') {
                score -= 10;
            }
        }
        Language::Javascript => {
            if code.contains("console.log(") && !code.contains(");") {
                score -= 10;
            }
        }
        _ => {}
    }

    score.clamp(0, 100)
}

fn score_structure(code: &str) -> u32 {
    let mut score = 100;

    let lines: Vec<&str> = code.lines().collect();
    let unique: HashSet<_> = lines.iter().collect();

    if unique.len() < lines.len() {
        score -= 10;
    }

    if code.matches("function").count() > 10 {
        score -= 10;
    }

    score.clamp(0, 100)
}

#[derive(Serialize)]
struct ScoreResult {
    cleanliness: u32,
    correctness: u32,
    structure: u32,
    overall: u32,
}
async fn judge_get() -> Html<&'static str> {
    Html(r#"
        <form method="POST" action="/judge">
          <textarea name="code" rows="8" cols="60">// paste code here</textarea><br>
          <button type="submit">Judge</button>
        </form>
    "#)
}

let app = Router::new()
    .route("/judge", get(judge_get).post(judge_handler)) // allow both
    .layer(cors);


#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/judge", post(judge_handler))
        .layer(cors);

    println!("🚀 Rust server running at http://localhost:3000/judge");

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

async fn judge_handler(req: Request) -> impl IntoResponse {
    let bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    let lang = detect_language(&body);
    let c1 = score_cleanliness(&body);
    let c2 = score_correctness(&body, &lang);
    let c3 = score_structure(&body);

    let result = ScoreResult {
        cleanliness: c1,
        correctness: c2,
        structure: c3,
        overall: (c1 + c2 + c3) / 3,
    };

    (StatusCode::OK, Json(result))
}