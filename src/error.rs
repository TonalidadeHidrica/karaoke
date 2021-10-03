use cpal::BuildStreamError;
use cpal::PlayStreamError;
use cpal::SupportedStreamConfigsError;
use druid::PlatformError;
use thiserror::Error;

use crate::config::ConfigLoadError;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Error while loading config: {0}")]
    ConfigLoadError(#[from] ConfigLoadError),
    #[error("Error while initializing GUI widget: {0}")]
    DruidError(#[from] PlatformError),
    #[error("Error while initializing audio: {0}")]
    AudioError(#[from] AudioError),
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("{0}")]
    WithMessage(&'static str),
    #[error("{0}")]
    SupportedStreamConfigsError(#[from] SupportedStreamConfigsError),
    #[error("{0}")]
    BuildStreamError(#[from] BuildStreamError),
    #[error("{0}")]
    PlayStreamError(#[from] PlayStreamError),
}
