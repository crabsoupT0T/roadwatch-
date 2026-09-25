use crate::error::AppError;
use crate::models::{severity_for, PotholeOut, ReportRequest, ReportResponse};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const MATCH_RADIUS_METERS: f64 = 10.0;
const MIN_PHONE_DIGITS: usize = 7;

fn normalize_phone(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn hash_phone(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(FromRow)]
struct PotholeRow {
    id: Uuid,
    lat: f64,
    lng: f64,
    photo: String,
    note: String,
    votes: i32,
    first_reported: chrono::DateTime<chrono::Utc>,
}

pub async fn list_potholes(pool: &PgPool) -> Result<Vec<PotholeOut>, AppError> {
    let rows: Vec<PotholeRow> = sqlx::query_as(
        r#"
        SELECT id,
               ST_Y(location::geometry) AS lat,
               ST_X(location::geometry) AS lng,
               photo, note, votes, first_reported
        FROM potholes
        ORDER BY votes DESC, first_reported ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PotholeOut {
            id: r.id,
            lat: r.lat,
            lng: r.lng,
            photo: r.photo,
            note: r.note,
            votes: r.votes,
            severity: severity_for(r.votes).to_string(),
            first_reported: r.first_reported,
        })
        .collect())
}

#[derive(FromRow)]
struct NearbyRow {
    id: Uuid,
    #[allow(dead_code)]
    votes: i32,
}

#[derive(FromRow)]
struct IdRow {
    id: Uuid,
}

#[derive(FromRow)]
struct VotesRow {
    votes: i32,
}

pub async fn report_pothole(pool: &PgPool, req: ReportRequest) -> Result<ReportResponse, AppError> {
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(AppError::Validation("invalid coordinates".into()));
    }
    if req.photo.trim().is_empty() {
        return Err(AppError::Validation("photo is required".into()));
    }
    let phone_digits = normalize_phone(&req.phone);
    if phone_digits.len() < MIN_PHONE_DIGITS {
        return Err(AppError::Validation(
            "a valid phone number is required".into(),
        ));
    }
    let phone_hash = hash_phone(&phone_digits);
    let note = req.note.unwrap_or_default();

    let mut tx = pool.begin().await?;

    let nearby: Option<NearbyRow> = sqlx::query_as(
        r#"
        SELECT id, votes
        FROM potholes
        WHERE ST_DWithin(
            location,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
            $3
        )
        ORDER BY ST_Distance(
            location,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography
        ) ASC
        LIMIT 1
        "#,
    )
    .bind(req.lng)
    .bind(req.lat)
    .bind(MATCH_RADIUS_METERS)
    .fetch_optional(&mut *tx)
    .await?;

    let response = if let Some(existing) = nearby {
        let inserted: Option<IdRow> = sqlx::query_as(
            r#"
            INSERT INTO votes (pothole_id, phone_hash)
            VALUES ($1, $2)
            ON CONFLICT (pothole_id, phone_hash) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(existing.id)
        .bind(&phone_hash)
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_none() {
            return Err(AppError::AlreadyVoted);
        }

        let updated: VotesRow = sqlx::query_as(
            r#"
            UPDATE potholes SET votes = votes + 1 WHERE id = $1
            RETURNING votes
            "#,
        )
        .bind(existing.id)
        .fetch_one(&mut *tx)
        .await?;

        ReportResponse {
            id: existing.id,
            votes: updated.votes,
            status: "confirmed".to_string(),
            severity: severity_for(updated.votes).to_string(),
        }
    } else {
        let new_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO potholes (id, location, photo, note, votes)
            VALUES ($1, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography, $4, $5, 1)
            "#,
        )
        .bind(new_id)
        .bind(req.lng)
        .bind(req.lat)
        .bind(&req.photo)
        .bind(&note)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO votes (pothole_id, phone_hash) VALUES ($1, $2)
            "#,
        )
        .bind(new_id)
        .bind(&phone_hash)
        .execute(&mut *tx)
        .await?;

        ReportResponse {
            id: new_id,
            votes: 1,
            status: "created".to_string(),
            severity: severity_for(1).to_string(),
        }
    };

    tx.commit().await?;
    Ok(response)
}
