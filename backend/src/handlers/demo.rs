use axum::response::Html;

pub async fn demo_form() -> Html<&'static str> {
    Html(r#"<!doctype html><meta charset="utf-8" />
    <body style="font-family: system-ui; margin: 2rem;">
      <h1>Judge (POST /judge)</h1>
      <p>Use curl or your frontend to POST raw text to <code>/judge</code>.</p>
    </body>"#)
}