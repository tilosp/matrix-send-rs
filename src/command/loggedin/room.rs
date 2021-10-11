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
    room::Room,
    ruma::events::room::message::{
        EmoteMessageEventContent, MessageType, NoticeMessageEventContent, TextMessageEventContent,
    },
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
        client.join_room_by_id_or_alias(&self.room, &self.servers).await?;
        Ok(())
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
    group = ArgGroup::with_name("type"),
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

    /// Send notice
    #[structopt(long, group = "type")]
    notice: bool,

    /// Send emote
    #[structopt(long, group = "type")]
    emote: bool,
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
        let (msg, markdown) = if let Some(language) = self.code {
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
            (fmt_msg, true)
        } else {
            (msg, self.markdown)
        };
        let content = if self.notice {
            MessageType::Notice(if markdown {
                NoticeMessageEventContent::markdown(msg)
            } else {
                NoticeMessageEventContent::plain(msg)
            })
        } else if self.emote {
            MessageType::Emote(if markdown {
                EmoteMessageEventContent::markdown(msg)
            } else {
                EmoteMessageEventContent::plain(msg)
            })
        } else {
            MessageType::Text(if markdown {
                TextMessageEventContent::markdown(msg)
            } else {
                TextMessageEventContent::plain(msg)
            })
        };
        client.send(&self.room, content).await?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct ListCommand {
    /// Kind
    #[structopt(possible_values = &["all", "joined", "invited", "left"], default_value = "joined")]
    kind: Vec<Kind>,
}

arg_enum! {
    #[derive(Debug)]
    enum Kind {
        All,
        Joined,
        Invited,
        Left
    }
}

impl ListCommand {
    async fn run(self, client: MatrixClient) -> Result {
        for room in client.rooms().into_iter().filter(|r| {
            self.kind.iter().any(|k| {
                matches!(
                    (k, r),
                    (Kind::All, _)
                        | (Kind::Joined, Room::Joined(_))
                        | (Kind::Left, Room::Left(_))
                        | (Kind::Invited, Room::Invited(_))
                )
            })
        }) {
            if let Ok(name) = room.display_name().await {
                println!("{}\t{}", room.room_id(), name);
            } else {
                println!("{}", room.room_id());
            }
        }
        Ok(())
    }
}
