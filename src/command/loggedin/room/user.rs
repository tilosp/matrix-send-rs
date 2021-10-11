use crate::{matrix::MatrixClient, Result};

use std::cmp::Reverse;

use structopt::StructOpt;

use matrix_sdk::ruma::identifiers::{RoomId, UserId};

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Kick a user
    Kick(KickCommand),

    /// Ban a user
    Ban(BanCommand),

    /// List users
    List(ListCommand),

    /// Invite user
    Invite(InviteCommand),
}

impl Command {
    pub(super) async fn run(self, client: MatrixClient, room: RoomId) -> Result {
        match self {
            Self::Kick(command) => command.run(client, room).await,
            Self::Ban(command) => command.run(client, room).await,
            Self::List(command) => command.run(client, room).await,
            Self::Invite(command) => command.run(client, room).await,
        }
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct KickCommand {
    /// User ID
    user: UserId,

    /// Reason for kick
    reason: Option<String>,
}

impl KickCommand {
    async fn run(self, client: MatrixClient, room: RoomId) -> Result {
        client
            .joined_room(&room)?
            .kick_user(&self.user, self.reason.as_deref())
            .await?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct BanCommand {
    /// User ID
    user: UserId,

    /// Reason for ban
    reason: Option<String>,
}

impl BanCommand {
    async fn run(self, client: MatrixClient, room: RoomId) -> Result {
        client
            .joined_room(&room)?
            .ban_user(&self.user, self.reason.as_deref())
            .await?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct ListCommand {}

impl ListCommand {
    async fn run(self, client: MatrixClient, room: RoomId) -> Result {
        let mut members = client.joined_room(&room)?.joined_members().await?;

        members.sort_by_key(|m| Reverse(m.power_level()));

        for member in members {
            if let Some(name) = member.display_name() {
                println!("{}\t{}\t{}", member.user_id(), member.power_level(), name);
            } else {
                println!("{}\t{}", member.user_id(), member.power_level());
            }
        }
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct InviteCommand {
    /// User ID
    user: UserId,
}

impl InviteCommand {
    async fn run(self, client: MatrixClient, room: RoomId) -> Result {
        client
            .joined_room(&room)?
            .invite_user_by_id(&self.user)
            .await?;
        Ok(())
    }
}
