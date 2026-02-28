use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub max_line_len: usize,
    pub max_body_bytes: usize,
}

impl AppState {
    pub fn new(max_line_len: usize, max_body_bytes: usize) -> Arc<Self> {
        Arc::new(Self { max_line_len, max_body_bytes })
    }
}