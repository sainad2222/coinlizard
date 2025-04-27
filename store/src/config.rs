/// Configuration for the InfluxDB store
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// InfluxDB server URL
    pub url: String,
    /// InfluxDB authentication token
    pub token: String,
    /// InfluxDB organization
    pub org: String,
    /// InfluxDB bucket to use for storing data
    pub bucket: String,
}

impl StoreConfig {
    /// Create a new store configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("INFLUXDB_URL")
            .map_err(|_| "INFLUXDB_URL environment variable not set")?;
        let token = std::env::var("INFLUXDB_TOKEN")
            .map_err(|_| "INFLUXDB_TOKEN environment variable not set")?;
        let org = std::env::var("INFLUXDB_ORG")
            .map_err(|_| "INFLUXDB_ORG environment variable not set")?;
        let bucket = std::env::var("INFLUXDB_BUCKET")
            .map_err(|_| "INFLUXDB_BUCKET environment variable not set")?;

        Ok(Self {
            url,
            token,
            org,
            bucket,
        })
    }
} 