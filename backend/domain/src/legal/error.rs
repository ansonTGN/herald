use crate::common::entities::app_errors::CoreError;

/// Domain-level errors for legal agreement / consent operations.
///
/// Implementations convert these into `CoreError` (see the `From` impl below)
/// before crossing into application/HTTP layers.
#[derive(Debug, thiserror::Error)]
pub enum LegalError {
    #[error("agreement version not found")]
    VersionNotFound,
    #[error("specified agreement version is not the current effective version")]
    StaleVersion,
    #[error("no access to agreements in this realm")]
    Forbidden,
    #[error("no draft saved for this agreement type")]
    DraftNotFound,
}

impl From<LegalError> for CoreError {
    fn from(err: LegalError) -> Self {
        match err {
            LegalError::VersionNotFound => CoreError::NotFound,
            // Stale version on revert → 409 Conflict (caller must re-read
            // current effective version before retrying).
            LegalError::StaleVersion => CoreError::Conflict(err.to_string()),
            LegalError::Forbidden => CoreError::Forbidden(err.to_string()),
            // Publish-from-draft with no staged draft → 404 (caller must save a
            // draft first). Discarding a missing draft is idempotent and does
            // not surface this error.
            LegalError::DraftNotFound => CoreError::NotFound,
        }
    }
}
