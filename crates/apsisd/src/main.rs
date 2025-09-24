// Apsis
// Copyright (C) 2025 Throneless Tech

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

mod api;
mod db;
mod error;
mod utils;

use axum::{
    Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use directories::ProjectDirs;
use error::{ApsisErrorKind, Result};
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use figment_file_provider_adapter::FileAdapter;
use mainline::Dht;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{metrics::SdkMeterProvider, trace::SdkTracer};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use subtle::ConstantTimeEq;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_log::AsTrace;
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::prelude::*;

use api::ApiState;

/// Apsis is a global Content-Addressed Store for the open web.
#[derive(Debug, Parser, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Verbosity
    #[command(flatten)]
    verbose: Verbosity,

    /// IP address and port to bind to
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    bind: Option<String>,

    /// API authorization token
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    auth: Option<String>,

    /// Path to Rocksdb database file
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    database: Option<String>,

    /// Enable Opentelemetry
    #[arg(short, long)]
    opentelemetry: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Verbosity
    verbose: Verbosity,

    /// IP address and port to bind to
    bind: String,

    /// API authorization token
    auth: String,

    /// Path to Oxigraph database file
    database: String,

    /// Enable Opentelemetry
    opentelemetry: bool,
}

async fn authenticate(
    State(state): State<ApiState>,
    req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    //Only the content endpoint is authenticated
    if !(req.uri() == "/content" || req.uri() == "/content/") {
        return Ok(next.run(req).await);
    }
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) if auth_header.as_bytes().ct_eq(state.auth.as_bytes()).into() => {
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn telemetry_tracer_init() -> Result<SdkTracer> {
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder().with_http();

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter.build()?)
        .build();

    Ok(tracer_provider.tracer("apsis_tracer"))
}

fn telemetry_meter_init() -> Result<SdkMeterProvider> {
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder().with_http();

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter.build()?)
        .build();

    Ok(meter_provider)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set project directories
    let proj_dirs = ProjectDirs::from("tech", "throneless", "apsis").ok_or(
        ApsisErrorKind::Directory("Failed to find project directories.".to_owned()),
    )?;

    // Merge the configuration from CLI, environment, files, container secrets
    let server: Config = Figment::new()
        .merge(FileAdapter::wrap(Toml::file(
            proj_dirs.config_dir().join("config.toml"),
        )))
        .merge(FileAdapter::wrap(Env::prefixed("APSIS_")))
        .merge(Serialized::defaults(Cli::parse()))
        .extract()?;

    // Setup logging and telemetry
    if server.opentelemetry {
        tracing_subscriber::registry()
            .with(server.verbose.log_level_filter().as_trace())
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_opentelemetry::layer().with_tracer(telemetry_tracer_init()?))
            .with(MetricsLayer::new(telemetry_meter_init()?))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(server.verbose.log_level_filter().as_trace())
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Initialize database
    let store = db::Db::try_open(&server.database.into())?;

    // Initialize DHT
    let dht = Dht::client()?;

    // Start RNG
    let rng = ChaCha20Rng::from_os_rng();

    // Create API state
    let token = CancellationToken::new();
    let tracker = TaskTracker::new();
    let state = ApiState {
        auth: server.auth,
        dht,
        rng,
        store,
        token: token.clone(),
        tracker: tracker.clone(),
    };

    // Run client API
    let app = Router::new()
        .route("/uri-res/N2R", get(api::name_to_resource))
        .route("/uri-res/R2N", post(api::resource_to_name))
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate))
        .with_state(state);

    println!("Server is running 🤖");

    {
        let tracker = tracker.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for signal");
            tracker.close();
            token.cancel();
        });
    }

    if let Ok(addr) = server.bind.parse::<SocketAddr>() {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Unable to bind to address");
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { tracker.wait().await })
        .await?;
    } else {
        let Ok(path) = server.bind.parse::<PathBuf>();
        let _ = tokio::fs::remove_file(&path).await;
        let listener = tokio::net::UnixListener::bind(path).expect("Unable to bind to address");
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async move { tracker.wait().await })
            .await?;
    };

    Ok(())
}
