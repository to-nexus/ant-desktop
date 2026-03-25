pub mod figma_check;

#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error("health check failed: {0}")]
    CheckFailed(String),
}
