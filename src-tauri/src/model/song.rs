use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::instrument::InstrumentBank;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SongMetadata {
    pub name: String,
    pub tempo: f64,
    pub time_signature: (u8, u8),
    pub ticks_per_beat: u32,
    pub driver_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: Uuid,
    pub name: String,
    pub channel: ChannelAssignment,
    pub instrument_id: Option<Uuid>,
    pub regions: Vec<Region>,
    pub muted: bool,
    pub solo: bool,
    pub volume: u8,
    pub pan: Pan,
    #[serde(default)]
    pub pitch_offset: i8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulation: Option<TrackModulation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackModulation {
    pub wait: u8,
    pub speed: u8,
    pub delta: u8,
    pub steps: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum ChannelAssignment {
    Fm(u8),
    Psg(u8),
    PsgNoise,
    Dac(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum Pan {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub id: Uuid,
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub notes: Vec<Note>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub tick: u64,
    pub pitch: u8,
    pub velocity: u8,
    pub duration_ticks: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<Uuid>,
    #[serde(default)]
    pub detune: i8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan_override: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulation: Option<NoteModulation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteModulation {
    pub wait: u8,
    pub speed: u8,
    pub delta: u8,
    pub steps: u8,
}

/// On-disk format for project.json (no instruments — they're separate files).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub metadata: SongMetadata,
    pub tracks: Vec<Track>,
}

/// Full in-memory song representation including instruments.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Song {
    pub metadata: SongMetadata,
    pub tracks: Vec<Track>,
    pub instruments: InstrumentBank,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_metadata_json_round_trip() {
        let meta = SongMetadata {
            name: "Test Song".into(),
            tempo: 140.0,
            time_signature: (4, 4),
            ticks_per_beat: 480,
            driver_id: "flamedriver".into(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"ticksPerBeat\":480"));
        assert!(json.contains("\"driverId\":\"flamedriver\""));
        let parsed: SongMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tempo, 140.0);
        assert_eq!(parsed.ticks_per_beat, 480);
    }

    #[test]
    fn test_project_file_json_round_trip() {
        let pf = ProjectFile {
            metadata: SongMetadata {
                name: "Test".into(),
                tempo: 120.0,
                time_signature: (3, 4),
                ticks_per_beat: 480,
                driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(),
                name: "FM1".into(),
                channel: ChannelAssignment::Fm(0),
                instrument_id: None,
                regions: vec![],
                muted: false,
                solo: false,
                volume: 100,
                pan: Pan::Center,
                pitch_offset: 0,
                modulation: None,
            }],
        };
        let json = serde_json::to_string_pretty(&pf).unwrap();
        let parsed: ProjectFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tracks.len(), 1);
        assert_eq!(parsed.tracks[0].name, "FM1");
        assert!(matches!(parsed.tracks[0].channel, ChannelAssignment::Fm(0)));
    }

    #[test]
    fn test_channel_assignment_serialization() {
        let fm = ChannelAssignment::Fm(3);
        let json = serde_json::to_string(&fm).unwrap();
        assert_eq!(json, r#"{"Fm":3}"#);

        let noise = ChannelAssignment::PsgNoise;
        let json = serde_json::to_string(&noise).unwrap();
        assert_eq!(json, r#""PsgNoise""#);
    }
}
