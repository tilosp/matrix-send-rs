use crate::dir::Directories;
use crate::matrix::MatrixClient;

use structopt::StructOpt;

use thiserror::Error;

mod command;
mod dir;
mod matrix;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(subcommand)]
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
    let Opt { command } = Opt::from_args();

    let dirs = Directories::new()?;

    let client = MatrixClient::load(&dirs).await;

    command.run(client, &dirs).await
}
