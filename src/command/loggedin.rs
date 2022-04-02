use crate::{matrix::MatrixClient, Result};

use clap::Parser;

mod room;

#[derive(Debug, Parser)]
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

#[derive(Debug, Parser)]
pub(crate) struct RoomCommand {
    #[clap(subcommand)]
    command: room::Command,
}

impl RoomCommand {
    async fn run(self, client: MatrixClient) -> Result {
        self.command.run(client).await
    }
}
