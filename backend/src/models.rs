use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct JudgeReq {
    pub code: String,
    pub lang: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JudgeResp {
    pub cleanliness: u32,
    pub correctness: u32,
    pub structure: u32,
    pub overall: u32,
    pub notes: Vec<Note>,
}

#[derive(Debug, Serialize)]
pub struct Note {
    pub line: usize,
    pub kind: String,
    pub severity: String, // "info" | "warn" | "error"
    pub msg: String,
}