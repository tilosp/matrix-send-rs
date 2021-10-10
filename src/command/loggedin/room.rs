use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::{matrix::MatrixClient, Error, Result};

use atty::Stream;

use structopt::{
    clap::{arg_enum, ArgGroup},
    StructOpt,
};

use matrix_sdk::{
    ruma::events::room::message::TextMessageEventContent,
    ruma::identifiers::{RoomId, RoomIdOrAliasId, ServerName},
};

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Join Room
    Join(JoinCommand),
    /// Leave Room
    Leave(LeaveCommand),
    /// Send Message into Room
    Send(SendCommand),
    /// List Rooms
    List(ListCommand),
}

impl Command {
    pub(super) async fn run(self, client: MatrixClient) -> Result {
        match self {
            Self::Join(command) => command.run(client).await,
            Self::List(command) => command.run(client).await,
            Self::Send(command) => command.run(client).await,
            Self::Leave(command) => command.run(client).await,
        }
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct JoinCommand {
    /// Alias or ID of Room
    room: RoomIdOrAliasId,
    /// Homeservers used to find the Room
    servers: Vec<Box<ServerName>>,
}

impl JoinCommand {
    async fn run(self, client: MatrixClient) -> Result {
        client.join_room(&self.room, &self.servers).await
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct LeaveCommand {
    /// Room ID
    room: RoomId,
}

impl LeaveCommand {
    async fn run(self, client: MatrixClient) -> Result {
        client
            .get_joined_room(&self.room)
            .ok_or(Error::InvalidRoom)?
            .leave()
            .await?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    group = ArgGroup::with_name("msgopt"),
    group = ArgGroup::with_name("format"),
)]
pub(crate) struct SendCommand {
    /// Room ID
    room: RoomId,
    /// Message to send
    #[structopt(group = "msgopt")]
    message: Option<String>,

    /// Read Message from file
    #[structopt(short, long, group = "msgopt")]
    file: Option<PathBuf>,

    /// Put message in code block
    #[structopt(name = "language", long = "code", group = "format")]
    code: Option<Option<String>>,

    /// Message is Markdown
    #[structopt(long, group = "format")]
    markdown: bool,
}

impl SendCommand {
    async fn run(self, client: MatrixClient) -> Result {
        let msg = if let Some(msg) = self.message {
            msg
        } else if let Some(file) = self.file {
            fs::read_to_string(file)?
        } else {
            let mut line = String::new();
            if atty::is(Stream::Stdin) {
                println!("Message:");
                io::stdin().read_line(&mut line)?;
            } else {
                io::stdin().read_to_string(&mut line)?;
            }
            line
        };
        client
            .send(
                &self.room,
                if let Some(language) = self.code {
                    let mut fmt_msg = String::from("```");
                    if let Some(language) = language {
                        fmt_msg.push_str(&language);
                    }
                    fmt_msg.push('\n');
                    fmt_msg.push_str(&msg);
                    if !fmt_msg.ends_with('\n') {
                        fmt_msg.push('\n');
                    }
                    fmt_msg.push_str("```");
                    TextMessageEventContent::markdown(fmt_msg)
                } else if self.markdown {
                    TextMessageEventContent::markdown(msg)
                } else {
                    TextMessageEventContent::plain(msg)
                },
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct ListCommand {
    /// Kind
    #[structopt(possible_values = &["joined", "invited", "left"], default_value = "joined")]
    kind: Kind,
}

arg_enum! {
    #[derive(Debug)]
    enum Kind {
        Joined,
        Invited,
        Left
    }
}

impl ListCommand {
    async fn run(self, client: MatrixClient) -> Result {
        let rooms = match self.kind {
            Kind::Joined => client.joined_rooms().await,
            Kind::Invited => client.invited_rooms().await,
            Kind::Left => client.left_rooms().await,
        };
        for room in rooms {
            if let Ok(name) = room.display_name().await {
                println!("{}\t{}", room.room_id(), name);
            } else {
                println!("{}", room.room_id());
            }
        }
        Ok(())
    }
}
