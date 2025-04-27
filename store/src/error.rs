use thiserror::Error;
use influxdb2::models::data_point::DataPointError;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("InfluxDB client error: {0}")]
    ClientError(String),

    #[error("InfluxDB query error: {0}")]
    QueryError(String),

    #[error("InfluxDB write error: {0}")]
    WriteError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Data conversion error: {0}")]
    ConversionError(String),

    #[error("InfluxDB error: {0}")]
    InfluxDbError(String),
}

impl From<StoreError> for common::Error {
    fn from(err: StoreError) -> Self {
        common::Error::DbError(err.to_string())
    }
}

impl From<influxdb2::RequestError> for StoreError {
    fn from(err: influxdb2::RequestError) -> Self {
        StoreError::InfluxDbError(err.to_string())
    }
}

impl From<DataPointError> for StoreError {
    fn from(err: DataPointError) -> Self {
        StoreError::WriteError(err.to_string())
    }
} 