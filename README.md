# CoinLizard

A simplified cryptocurrency data API service inspired by CoinGecko. This project fetches cryptocurrency price data from Coinbase and Binance APIs, and provides a REST API for accessing current and historical prices.

## Project Structure

The project is organized as a Cargo workspace with multiple crates:

- `api`: REST API service using Axum
- `store`: InfluxDB integration for storing and retrieving price data
- `connectors`: Exchange API clients (Coinbase, Binance)
- `common`: Shared utilities and data models

## Features

- Fetch current cryptocurrency prices from multiple exchanges (Coinbase, Binance)
- Retrieve historical price data with different time intervals
- Store time-series price data in InfluxDB
- RESTful API for accessing the data

## Prerequisites

- Rust and Cargo
- InfluxDB (v2.x) or Docker and Docker Compose

## Getting Started

### Running with Docker Compose

The easiest way to run the project is using Docker Compose, which will start both the API server and InfluxDB:

```bash
cd coinlizard
docker compose up
```

The API will be available at http://localhost:3000.

### Manual Setup

1. Start InfluxDB:

   You can either install InfluxDB locally or use Docker:

   ```bash
   docker run -d \
     --name influxdb \
     -p 8086:8086 \
     -e DOCKER_INFLUXDB_INIT_MODE=setup \
     -e DOCKER_INFLUXDB_INIT_USERNAME=admin \
     -e DOCKER_INFLUXDB_INIT_PASSWORD=password123 \
     -e DOCKER_INFLUXDB_INIT_ORG=coinlizard \
     -e DOCKER_INFLUXDB_INIT_BUCKET=coinlizard \
     -e DOCKER_INFLUXDB_INIT_ADMIN_TOKEN=my-super-secret-token \
     influxdb:2.7
   ```

2. Set environment variables:

   ```bash
   export INFLUXDB_URL=http://localhost:8086
   export INFLUXDB_TOKEN=my-super-secret-token
   export INFLUXDB_ORG=coinlizard
   export INFLUXDB_BUCKET=coinlizard
   export RUST_LOG=info
   ```

3. Build and run the API service:

   ```bash
   cd coinlizard
   cargo run -p api
   ```

## API Endpoints

### List Available Coins

```
GET /api/v1/coins
```

Returns a list of all supported cryptocurrencies.

### Get Current Price

```
GET /api/v1/coins/{id}/price?currency={currency}&exchange={exchange}
```

Parameters:
- `id`: Coin identifier (e.g., bitcoin, ethereum)
- `currency` (optional): Quote currency (default: USD)
- `exchange` (optional): Specific exchange to query (coinbase, binance)

Returns the current price of the specified coin.

### Get Historical Prices

```
GET /api/v1/coins/{id}/history/daily?currency={currency}&exchange={exchange}&interval={interval}&start={start_time}&end={end_time}&limit={limit}
```

Parameters:
- `id`: Coin identifier (e.g., bitcoin, ethereum)
- `currency` (optional): Quote currency (default: USD)
- `exchange` (optional): Specific exchange to query (coinbase, binance)
- `interval` (optional): Time interval (1m, 5m, 15m, 1h, 4h, 1d, 1w; default: 1d)
- `start` (optional): Start time in ISO format
- `end` (optional): End time in ISO format
- `limit` (optional): Maximum number of data points to return

Returns historical price data for the specified coin.

## Development

To run the project for development:

```bash
# Start InfluxDB in Docker
docker run -d \
  --name influxdb \
  -p 8086:8086 \
  -e DOCKER_INFLUXDB_INIT_MODE=setup \
  -e DOCKER_INFLUXDB_INIT_USERNAME=admin \
  -e DOCKER_INFLUXDB_INIT_PASSWORD=password123 \
  -e DOCKER_INFLUXDB_INIT_ORG=coinlizard \
  -e DOCKER_INFLUXDB_INIT_BUCKET=coinlizard \
  -e DOCKER_INFLUXDB_INIT_ADMIN_TOKEN=my-super-secret-token \
  influxdb:2.7

# Set environment variables
export INFLUXDB_URL=http://localhost:8086
export INFLUXDB_TOKEN=my-super-secret-token
export INFLUXDB_ORG=coinlizard
export INFLUXDB_BUCKET=coinlizard
export RUST_LOG=debug  # More verbose logging for development

# Run the API with hot reloading
cd coinlizard
cargo watch -x "run -p api"
```

## License

This project is licensed under the MIT License - see the LICENSE file for details. 