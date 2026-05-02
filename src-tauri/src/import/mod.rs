pub mod psg_envelopes;
pub mod smps_mapper;
pub mod smps_parser;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub metadata: crate::model::song::SongMetadata,
    pub track_count: usize,
    pub instrument_count: usize,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub channel: String,
    pub message: String,
}
