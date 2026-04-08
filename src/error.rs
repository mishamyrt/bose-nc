use thiserror::Error;

use crate::device;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("no paired Bose devices found")]
    NoDeviceFound,

    #[error("no Bose device matches '{filter}'. Available: {available}")]
    NoMatch { filter: String, available: String },

    #[error("NC level {level} exceeds maximum ({max}) for {product}")]
    NcLevelExceeded { level: u8, max: u8, product: String },

    #[error(transparent)]
    Device(#[from] device::DeviceError),

    #[error("output error: {0}")]
    Output(#[from] serde_json::Error),
}
