use crate::{matrix::MatrixClient, Result};

use structopt::StructOpt;

mod room;

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Room Subcommands
    Room(RoomCommand),
}

impl Command {
    pub(super) async fn run(self, client: MatrixClient) -> Result {
        match self {
            Self::Room(command) => command.run(client).await,
        }
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct RoomCommand {
    #[structopt(subcommand)]
    command: room::Command,
}

impl RoomCommand {
    async fn run(self, client: MatrixClient) -> Result {
        self.command.run(client).await
    }
}
