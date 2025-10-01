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

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing_log::AsTrace;
use url::Url;

/// The Apsis CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(version, about, long_about = None)]
struct Cli {
    /// IP address and port to connect to
    #[arg(short, long)]
    connect: String,

    /// Verbosity
    #[command(flatten)]
    verbose: Verbosity,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
struct Input {
    /// JSON data
    #[arg(short, long)]
    json: Option<String>,

    /// File path
    #[arg(short, long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
struct Output {
    /// JSON data
    #[arg(short, long)]
    stdout: bool,

    /// File path
    #[arg(short, long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Upload JSON or file data
    #[command(arg_required_else_help = true)]
    Upload {
        /// API authentication token
        #[arg(short, long)]
        auth: String,

        /// Input selection
        #[command(flatten)]
        input: Input,
    },

    /// Download JSON or file data
    #[command(arg_required_else_help = true)]
    Download {
        /// Output selection
        #[command(flatten)]
        output: Output,

        /// Capability URN
        #[arg(required = true)]
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

    let mut url = Url::parse(&connect).expect("Invalid connection URI.");
    url = url.join("uri-res/")?;
    let client = reqwest::Client::new();
    match args.command {
        Commands::Upload { auth, input } => {
            let url = url.join("R2N")?;
            if let Some(data) = input.json {
                let res = client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", auth)
                    .body(data)
                    .send()
                    .await?;
                println!("{}", res.text().await?);
            } else if let Some(path) = input.file {
                let file = File::open(path).await?;
                let res = client
                    .post(url)
                    .header("Authorization", auth)
                    .body(file)
                    .send()
                    .await?;
                println!("{}", res.text().await?);
            }
        }
        Commands::Download { output, urn } => {
            let route = "N2R?".to_owned() + &urn;
            let url = url.join(&route)?;
            if output.stdout {
                println!("{}", client.get(url).send().await?.text().await?);
            } else if let Some(path) = output.file {
                let mut file = File::create(&path).await?;
                file.write_all(&client.get(url).send().await?.bytes().await?)
                    .await?;
                file.flush().await?;
                println!("Wrote to file {}.", path.to_string_lossy());
            }
        }
    }
    Ok(())
}
