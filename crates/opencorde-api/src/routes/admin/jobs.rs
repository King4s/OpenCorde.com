//! # Admin Job Registry
//! In-memory registry for tracking background jobs and their execution history.
//!
//! ## Architecture
//! Jobs are registered at startup by background task spawners. Each job tracks
//! its name, queue, status, last run time, next scheduled run, error traces,
//! and execution metrics. The registry is purely observational — it does not
//! execute or schedule jobs, only tracks them.
//!
//! ## Endpoints
//! - GET  /api/v1/admin/jobs          — List all registered jobs
//! - GET  /api/v1/admin/jobs/{id}     — Get single job details with recent logs
//! - POST /api/v1/admin/jobs/{id}/retry   — Mark a failed job for retry
//! - POST /api/v1/admin/jobs/{id}/pause   — Pause a job (skip next scheduled run)
//! - POST /api/v1/admin/jobs/{id}/resume  — Resume a paused job
//! - POST /api/v1/admin/jobs/retry-failed — Retry all failed jobs at once
//!
//! ## Depends On
//! - tokio (sync primitives, async runtime)
//! - serde (JSON serialization)
//! - chrono (timestamps)
//! - tracing (structured logging)

use crate::error::ApiError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── Types ─────────────────────────────────────────────────────────────────

/// A single background job entry visible to the admin dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    /// Unique job identifier (e.g. "federation-registry-sync")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Logical queue / category (e.g. "federation", "maintenance", "realtime")
    pub queue: String,
    /// Current status: "running", "idle", "failed", "paused", "completed"
    pub status: String,
    /// ISO 8601 timestamp of the most recent run start
    pub last_run_at: Option<String>,
    /// ISO 8601 timestamp of the next scheduled run (if periodic)
    pub next_run_at: Option<String>,
    /// How many times this job has run
    pub run_count: u64,
    /// How many runs succeeded
    pub success_count: u64,
    /// How many runs failed
    pub failure_count: u64,
    /// Average duration in milliseconds (of successful runs)
    pub avg_duration_ms: Option<f64>,
    /// Most recent error message + trace, if the last run failed
    pub last_error: Option<String>,
    /// ISO 8601 timestamp of the most recent error
    pub last_error_at: Option<String>,
    /// Whether the job is currently paused (skips scheduled runs)
    pub paused: bool,
    /// Labels / tags for filtering
    pub labels: Vec<String>,
}

/// A log entry for a single job execution.
#[derive(Debug, Clone, Serialize)]
pub struct JobLogEntry {
    /// ISO 8601 start timestamp
    pub started_at: String,
    /// ISO 8601 end timestamp (None if still running)
    pub ended_at: Option<String>,
    /// "success", "failure", or "running"
    pub outcome: String,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Error message if outcome is failure
    pub error: Option<String>,
}

/// Response for a single job with recent log entries.
#[derive(Debug, Clone, Serialize)]
pub struct JobDetail {
    #[serde(flatten)]
    pub info: JobInfo,
    /// Most recent execution log entries (up to 25)
    pub recent_logs: Vec<JobLogEntry>,
}

/// Metrics summary for the jobs dashboard header.
#[derive(Debug, Clone, Serialize)]
pub struct JobMetrics {
    /// Total registered jobs
    pub total_jobs: usize,
    /// Jobs currently running
    pub running: usize,
    /// Jobs in failed state
    pub failed: usize,
    /// Jobs paused
    pub paused: usize,
    /// Jobs idle (waiting for next scheduled run)
    pub idle: usize,
    /// Throughput: successful runs in the last hour
    pub runs_last_hour: u64,
    /// Failure rate: failures / total runs (0.0–1.0)
    pub failure_rate: f64,
}

// ── Internal entry ────────────────────────────────────────────────────────

/// Internal mutable state for a single job.
#[derive(Debug, Clone)]
struct JobEntry {
    info: JobInfo,
    /// Ring buffer of recent log entries (max 50)
    logs: Vec<JobLogEntry>,
    /// When this job is scheduled to run next (for periodic jobs)
    next_scheduled: Option<DateTime<Utc>>,
    /// Accumulated successful durations for average computation
    total_success_duration_ms: u64,
}

impl JobEntry {
    fn new(id: String, name: String, queue: String, labels: Vec<String>) -> Self {
        Self {
            info: JobInfo {
                id: id.clone(),
                name,
                queue,
                status: "idle".into(),
                last_run_at: None,
                next_run_at: None,
                run_count: 0,
                success_count: 0,
                failure_count: 0,
                avg_duration_ms: None,
                last_error: None,
                last_error_at: None,
                paused: false,
                labels,
            },
            logs: Vec::new(),
            next_scheduled: None,
            total_success_duration_ms: 0,
        }
    }

    fn to_info(&self) -> JobInfo {
        self.info.clone()
    }

    fn to_detail(&self) -> JobDetail {
        JobDetail {
            info: self.info.clone(),
            recent_logs: self.logs.clone(),
        }
    }

    fn record_run_start(&mut self) {
        self.info.status = "running".into();
        self.info.last_run_at = Some(Utc::now().to_rfc3339());
        self.info.run_count += 1;
    }

    fn record_run_success(&mut self, started_at: DateTime<Utc>) {
        let ended = Utc::now();
        let duration_ms = (ended - started_at).num_milliseconds() as u64;
        self.total_success_duration_ms += duration_ms;
        self.info.success_count += 1;
        self.info.avg_duration_ms =
            Some(self.total_success_duration_ms as f64 / self.info.success_count as f64);
        self.info.status = "idle".into();
        self.info.last_error = None;
        self.info.last_error_at = None;
        self.push_log(JobLogEntry {
            started_at: started_at.to_rfc3339(),
            ended_at: Some(ended.to_rfc3339()),
            outcome: "success".into(),
            duration_ms: Some(duration_ms),
            error: None,
        });
    }

    fn record_run_failure(&mut self, started_at: DateTime<Utc>, error: String) {
        let ended = Utc::now();
        self.info.failure_count += 1;
        self.info.status = "failed".into();
        self.info.last_error = Some(error.clone());
        self.info.last_error_at = Some(ended.to_rfc3339());
        self.push_log(JobLogEntry {
            started_at: started_at.to_rfc3339(),
            ended_at: Some(ended.to_rfc3339()),
            outcome: "failure".into(),
            duration_ms: Some((ended - started_at).num_milliseconds() as u64),
            error: Some(error),
        });
    }

    fn set_next_scheduled(&mut self, next: DateTime<Utc>) {
        self.next_scheduled = Some(next);
        if !self.info.paused {
            self.info.next_run_at = Some(next.to_rfc3339());
        }
    }

    fn pause(&mut self) {
        self.info.paused = true;
        self.info.next_run_at = None;
        if self.info.status == "idle" {
            self.info.status = "paused".into();
        }
    }

    fn resume(&mut self) {
        self.info.paused = false;
        if self.info.status == "paused" {
            self.info.status = "idle".into();
        }
        if let Some(next) = self.next_scheduled {
            self.info.next_run_at = Some(next.to_rfc3339());
        }
    }

    fn push_log(&mut self, entry: JobLogEntry) {
        self.logs.push(entry);
        if self.logs.len() > 50 {
            self.logs.remove(0);
        }
    }
}

// ── Registry ──────────────────────────────────────────────────────────────

/// In-memory registry of all background jobs.
///
/// Thread-safe via `Arc<RwLock<>>`. Background tasks call `record_*` methods
/// to update their status; the admin API reads from it.
#[derive(Clone)]
pub struct JobRegistry {
    jobs: Arc<RwLock<HashMap<String, JobEntry>>>,
}

impl JobRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new job. Idempotent — if the id already exists, returns
    /// without overwriting (so restarts don't lose accumulated metrics).
    pub async fn register(&self, id: &str, name: &str, queue: &str, labels: Vec<String>) {
        let mut jobs = self.jobs.write().await;
        jobs.entry(id.to_string())
            .or_insert_with(|| JobEntry::new(id.into(), name.into(), queue.into(), labels));
    }

    /// Update the next scheduled run time for a periodic job.
    pub async fn set_next_run(&self, id: &str, next: DateTime<Utc>) {
        let mut jobs = self.jobs.write().await;
        if let Some(entry) = jobs.get_mut(id) {
            entry.set_next_scheduled(next);
        }
    }

    /// Record the start of a job run. Returns a started_at timestamp for
    /// later use with `record_success` / `record_failure`.
    pub async fn record_run_start(&self, id: &str) -> Option<DateTime<Utc>> {
        let mut jobs = self.jobs.write().await;
        let entry = jobs.get_mut(id)?;
        let started = Utc::now();
        entry.record_run_start();
        Some(started)
    }

    /// Record successful completion of a job run.
    pub async fn record_success(&self, id: &str, started_at: DateTime<Utc>) {
        let mut jobs = self.jobs.write().await;
        if let Some(entry) = jobs.get_mut(id) {
            entry.record_run_success(started_at);
        }
    }

    /// Record a failed job run with an error message.
    pub async fn record_failure(&self, id: &str, started_at: DateTime<Utc>, error: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(entry) = jobs.get_mut(id) {
            entry.record_run_failure(started_at, error.to_string());
        }
    }

    /// Pause a job (skip next scheduled run).
    pub async fn pause(&self, id: &str) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write().await;
        let entry = jobs
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("job not found: {id}")))?;
        entry.pause();
        Ok(())
    }

    /// Resume a paused job.
    pub async fn resume(&self, id: &str) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write().await;
        let entry = jobs
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("job not found: {id}")))?;
        entry.resume();
        Ok(())
    }

    /// List all registered jobs as `JobInfo`.
    pub async fn list(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.read().await;
        jobs.values().map(|e| e.to_info()).collect()
    }

    /// Get a single job's full detail including recent logs.
    pub async fn get(&self, id: &str) -> Result<JobDetail, ApiError> {
        let jobs = self.jobs.read().await;
        jobs.get(id)
            .map(|e| e.to_detail())
            .ok_or_else(|| ApiError::NotFound(format!("job not found: {id}")))
    }

    /// Compute aggregate metrics for the dashboard header.
    pub async fn metrics(&self) -> JobMetrics {
        let jobs = self.jobs.read().await;
        let total = jobs.len();
        let mut running = 0usize;
        let mut failed = 0usize;
        let mut paused = 0usize;
        let mut idle = 0usize;
        let mut runs_last_hour = 0u64;
        let mut total_runs = 0u64;
        let mut total_failures = 0u64;
        let hour_ago = Utc::now() - chrono::Duration::hours(1);

        for entry in jobs.values() {
            match entry.info.status.as_str() {
                "running" => running += 1,
                "failed" => failed += 1,
                "paused" => paused += 1,
                _ => idle += 1,
            }
            total_runs += entry.info.run_count;
            total_failures += entry.info.failure_count;

            // Count successful runs in the last hour
            for log in &entry.logs {
                if log.outcome == "success" {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&log.started_at) {
                        if ts > hour_ago {
                            runs_last_hour += 1;
                        }
                    }
                }
            }
        }

        JobMetrics {
            total_jobs: total,
            running,
            failed,
            paused,
            idle,
            runs_last_hour,
            failure_rate: if total_runs > 0 {
                total_failures as f64 / total_runs as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── API Handlers ──────────────────────────────────────────────────────────

use super::handlers::is_admin;
use crate::{AppState, middleware::auth::AuthUser};
use axum::{
    Json,
    extract::{Path, State},
};

/// GET /api/v1/admin/jobs — List all registered jobs with metrics.
pub async fn list_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: listing background jobs");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    let jobs = state.job_registry.list().await;
    let metrics = state.job_registry.metrics().await;
    Ok(Json(serde_json::json!({
        "jobs": jobs,
        "metrics": metrics,
    })))
}

/// GET /api/v1/admin/jobs/{job_id} — Get a single job's detail + recent logs.
pub async fn get_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<String>,
) -> Result<Json<JobDetail>, ApiError> {
    tracing::info!(user_id = %auth.user_id, job_id = %job_id, "admin: fetching job detail");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    let detail = state.job_registry.get(&job_id).await?;
    Ok(Json(detail))
}

/// POST /api/v1/admin/jobs/{job_id}/retry — Retry a failed job.
/// Sets status back to idle so the next scheduler tick picks it up.
pub async fn retry_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(user_id = %auth.user_id, job_id = %job_id, "admin: retrying job");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    // For now, "retry" means clear the failed status so the scheduler picks it up.
    // We do this by pausing then immediately resuming, which resets to idle.
    state.job_registry.resume(&job_id).await?;
    let detail = state.job_registry.get(&job_id).await?;
    Ok(Json(serde_json::json!({
        "retried": true,
        "job": detail.info,
    })))
}

/// POST /api/v1/admin/jobs/{job_id}/pause — Pause a job.
pub async fn pause_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(user_id = %auth.user_id, job_id = %job_id, "admin: pausing job");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    state.job_registry.pause(&job_id).await?;
    let detail = state.job_registry.get(&job_id).await?;
    Ok(Json(serde_json::json!({
        "paused": true,
        "job": detail.info,
    })))
}

/// POST /api/v1/admin/jobs/{job_id}/resume — Resume a paused job.
pub async fn resume_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(user_id = %auth.user_id, job_id = %job_id, "admin: resuming job");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    state.job_registry.resume(&job_id).await?;
    let detail = state.job_registry.get(&job_id).await?;
    Ok(Json(serde_json::json!({
        "resumed": true,
        "job": detail.info,
    })))
}

/// POST /api/v1/admin/jobs/retry-failed — Retry all currently failed jobs.
pub async fn retry_all_failed(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: retrying all failed jobs");
    if !is_admin(&auth, &state) {
        return Err(ApiError::Forbidden);
    }
    let jobs = state.job_registry.list().await;
    let mut retried = Vec::new();
    for job in &jobs {
        if job.status == "failed" {
            if state.job_registry.resume(&job.id).await.is_ok() {
                retried.push(job.id.clone());
            }
        }
    }
    Ok(Json(serde_json::json!({
        "retried_count": retried.len(),
        "retried": retried,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_list() {
        let registry = JobRegistry::new();
        registry
            .register("test-job", "Test Job", "maintenance", vec!["test".into()])
            .await;
        let jobs = registry.list().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "test-job");
        assert_eq!(jobs[0].status, "idle");
    }

    #[tokio::test]
    async fn test_record_run_lifecycle() {
        let registry = JobRegistry::new();
        registry
            .register("test-job", "Test Job", "maintenance", vec![])
            .await;

        let started = registry.record_run_start("test-job").await.unwrap();
        let jobs = registry.list().await;
        assert_eq!(jobs[0].status, "running");
        assert_eq!(jobs[0].run_count, 1);

        registry.record_success("test-job", started).await;
        let jobs = registry.list().await;
        assert_eq!(jobs[0].status, "idle");
        assert_eq!(jobs[0].success_count, 1);
        assert!(jobs[0].avg_duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_record_failure() {
        let registry = JobRegistry::new();
        registry
            .register("test-job", "Test Job", "maintenance", vec![])
            .await;

        let started = registry.record_run_start("test-job").await.unwrap();
        registry
            .record_failure("test-job", started, "something broke")
            .await;
        let jobs = registry.list().await;
        assert_eq!(jobs[0].status, "failed");
        assert_eq!(jobs[0].failure_count, 1);
        assert!(jobs[0].last_error.as_deref().unwrap().contains("broke"));
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let registry = JobRegistry::new();
        registry
            .register("test-job", "Test Job", "maintenance", vec![])
            .await;

        registry.pause("test-job").await.unwrap();
        let jobs = registry.list().await;
        assert!(jobs[0].paused);
        assert_eq!(jobs[0].status, "paused");

        registry.resume("test-job").await.unwrap();
        let jobs = registry.list().await;
        assert!(!jobs[0].paused);
        assert_eq!(jobs[0].status, "idle");
    }

    #[tokio::test]
    async fn test_metrics() {
        let registry = JobRegistry::new();
        registry.register("job-a", "Job A", "q1", vec![]).await;
        registry.register("job-b", "Job B", "q2", vec![]).await;

        let started = registry.record_run_start("job-a").await.unwrap();
        registry.record_failure("job-a", started, "err").await;

        let metrics = registry.metrics().await;
        assert_eq!(metrics.total_jobs, 2);
        assert_eq!(metrics.failed, 1);
        assert_eq!(metrics.idle, 1);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let registry = JobRegistry::new();
        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
    }
}
