//! # Welcome to matrix-send-rs
//!
//! Requirements: you must install openssl-devel in your operating system otherwise
//! cargo will not build. There are different names for this package depending on the
//! OS. E.g. `libssl-dev`, `openssl-devel`, etc.
//!
//! Files are created and installed in local data directory $XDG_DATA_HOME
//! according to XDG standard.
//! E.g. on Linux under /home/someuser/.local/share/matrix-send/
//!

use crate::dir::Directories;
use crate::matrix::MatrixClient;

use clap::Parser;

use thiserror::Error;

mod command;
mod dir;
mod matrix;

use tracing::{debug, enabled, Level};

// records an event outside of any span context:
const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Parser)]
struct Opt {
    #[clap(subcommand)]
    command: command::Command,
}

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    Custom(&'static str),

    #[error("No valid home directory path")]
    NoNomeDirectory,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Invalid Room")]
    InvalidRoom,

    #[error("Invalid File")]
    InvalidFile,

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Matrix(#[from] matrix_sdk::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Http(#[from] matrix_sdk::HttpError),
}

impl Error {
    pub(crate) fn custom<T>(message: &'static str) -> Result<T> {
        Err(Error::Custom(message))
    }
}

pub(crate) type Result<T = ()> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result {
    // set log level e.g. via RUST_LOG=matrix-send=DEBUG cargo run
    tracing_subscriber::fmt::init();
    if enabled!(Level::TRACE) {
        debug!("Log level is set to TRACE.");
    } else if enabled!(Level::DEBUG) {
        debug!("Log level is set to DEBUG.");
    }
    let Opt { command } = Opt::parse();

    let dirs = Directories::new()?;

    let client = MatrixClient::load(&dirs).await; // re-login

    command.run(client, &dirs).await
}
