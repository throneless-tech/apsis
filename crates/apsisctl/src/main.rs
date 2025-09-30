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

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use futures_util::{Sink, SinkExt, StreamExt};
use http::HeaderValue;
use serde_json::json;
use std::fs::File;
use std::io;
use std::{fs, marker::Unpin, path::PathBuf};
use tracing::{debug, error};
use tracing_log::AsTrace;
use url::Url;

/// The Apsis CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(version, about, long_about = None)]
struct Cli {
    /// API authentication token
    #[arg(short, long)]
    auth: String,

    /// IP address and port to connect to
    #[arg(short, long)]
    connect: String,

    /// Verbosity
    #[command(flatten)]
    verbose: Verbosity,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Insert JSON data
    #[command(arg_required_else_help = true)]
    AddJson {
        /// JSON data
        #[arg(required = true)]
        data: String,
    },

    /// Insert binary file
    #[command(arg_required_else_help = true)]
    AddFile {
        /// File path
        #[arg(required = true)]
        path: PathBuf,
    },

    /// Fetch JSON
    #[command(arg_required_else_help = true)]
    GetJson {
        /// Capability URN
        #[arg(short, long)]
        urn: String,
    },

    /// Fetch binary file
    #[command(arg_required_else_help = true)]
    GetFile {
        /// Capability URN
        #[arg(short, long)]
        urn: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbose.log_level_filter().as_trace())
        .init();
    let connect = args.connect;
    let auth = args.auth;

    let url = Url::parse(&connect).unwrap();
    let client = reqwest::Client::new();
    match args.command {
        Commands::AddJson { data } => {
            debug!("Request: {:?}", data.to_string());
        }
        Commands::AddFile { path } => {
            debug!("Request: {:?}", path);
            let file = File::open(path)?;
            let res = client.post(url).body(file).send().await?;
        }
        Commands::GetJson { urn } => {
            debug!("Request: {:?}", urn.to_string());
            let res = client.get(url).send().await?.json().await?;
            println!("{}", res);
        }
        Commands::GetFile { urn } => {
            debug!("Request: {:?}", urn.to_string());
            let res = client.get(url).send().await?.bytes().await?;
            println!("{:?}", res);
        }
    }
    Ok(())
}
