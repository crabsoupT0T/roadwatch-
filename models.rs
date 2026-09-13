use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Body of POST /api/report.
#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub lat: f64,
    pub lng: f64,
    pub photo: String,
    pub note: Option<String>,
    pub phone: String,
}

/// A pothole as returned by GET /api/potholes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotholeOut {
    pub id: Uuid,
    pub lat: f64,
    pub lng: f64,
    pub photo: String,
    pub note: String,
    pub votes: i32,
    pub severity: String,
    pub first_reported: DateTime<Utc>,
}

/// Response for POST /api/report.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResponse {
    pub id: Uuid,
    pub votes: i32,
    /// "created" when this started a new pothole, "confirmed" when it added
    /// a vote to one that already existed within the match radius.
    pub status: String,
    pub severity: String,
}

pub fn severity_for(votes: i32) -> &'static str {
    if votes >= 5 {
        "priority"
    } else if votes >= 2 {
        "confirmed"
    } else {
        "reported"
    }
}
