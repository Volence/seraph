pub mod smps;
pub mod vgm;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportError {
    pub track_name: String,
    pub region_index: Option<usize>,
    pub note_index: Option<usize>,
    pub message: String,
}
