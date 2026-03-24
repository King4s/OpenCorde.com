//! # Recording Types
//! Request/response types and database row structs for the recordings module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Public response types ────────────────────────────────────────────────────

/// A single recording entry returned to the client.
#[derive(Debug, Serialize)]
pub struct RecordingResponse {
    pub id: String,
    pub server_id: String,
    pub channel_id: String,
    pub started_by: String,
    pub egress_id: String,
    pub status: String,
    pub file_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
}

/// Response returned after successfully starting a recording.
#[derive(Debug, Serialize)]
pub struct StartRecordingResponse {
    pub recording_id: String,
    pub egress_id: String,
    pub status: String,
}

// ─── LiveKit Egress API wire types ────────────────────────────────────────────

/// MinIO/S3 upload configuration sent to LiveKit Egress.
#[derive(Debug, Serialize)]
pub struct EgressS3Config {
    pub access_key: String,
    pub secret: String,
    pub region: String,
    pub endpoint: String,
    pub bucket: String,
    pub force_path_style: bool,
}

/// File output configuration for the Egress job.
#[derive(Debug, Serialize)]
pub struct EgressFileOutput {
    pub filepath: String,
    pub s3: EgressS3Config,
}

/// Body sent to LiveKit POST /egress/room.
#[derive(Debug, Serialize)]
pub struct EgressStartRequest {
    pub room_name: String,
    pub file: EgressFileOutput,
}

/// LiveKit Egress start response (we only need egress_id).
#[derive(Debug, Deserialize)]
pub struct EgressStartResponse {
    pub egress_id: String,
}

/// Body sent to LiveKit POST /egress/stop.
#[derive(Debug, Serialize)]
pub struct EgressStopRequest {
    pub egress_id: String,
}

// ─── Database row ─────────────────────────────────────────────────────────────

/// sqlx row type matching the `recordings` table.
#[derive(sqlx::FromRow)]
pub struct RecordingRow {
    pub id: i64,
    pub server_id: i64,
    pub channel_id: i64,
    pub started_by: i64,
    pub egress_id: String,
    pub status: String,
    pub file_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
}

/// Convert a DB row to the public API response shape.
pub fn row_to_response(r: RecordingRow) -> RecordingResponse {
    RecordingResponse {
        id: r.id.to_string(),
        server_id: r.server_id.to_string(),
        channel_id: r.channel_id.to_string(),
        started_by: r.started_by.to_string(),
        egress_id: r.egress_id,
        status: r.status,
        file_path: r.file_path,
        started_at: r.started_at,
        stopped_at: r.stopped_at,
    }
}
