use axum::{extract::State, http::StatusCode, Json};

use crate::{
    db,
    error::AppError,
    models::{PotholeOut, ReportRequest, ReportResponse},
    AppState,
};

pub async fn health() -> &'static str {
    "ok"
}

pub async fn list_potholes(
    State(state): State<AppState>,
) -> Result<Json<Vec<PotholeOut>>, AppError> {
    let potholes = db::list_potholes(&state.pool).await?;
    Ok(Json(potholes))
}

pub async fn report_pothole(
    State(state): State<AppState>,
    Json(req): Json<ReportRequest>,
) -> Result<(StatusCode, Json<ReportResponse>), AppError> {
    let result = db::report_pothole(&state.pool, req).await?;
    let status = if result.status == "created" {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(result)))
}
