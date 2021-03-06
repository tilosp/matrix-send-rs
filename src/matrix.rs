use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use crate::dir::Directories;
use crate::{Error, Result};

use matrix_sdk::{
    api::r0::session::login::Response as LoginResponse,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        AnyMessageEventContent,
    },
    identifiers::{DeviceId, RoomId, RoomIdOrAliasId, ServerName, UserId},
    locks::RwLock,
    Client, ClientConfig, Room, Session, SyncSettings,
};
use url::Url;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SessionData {
    homeserver: Url,
    access_token: String,
    device_id: Box<DeviceId>,
    user_id: UserId,
}

impl SessionData {
    fn load(path: &PathBuf) -> Result<SessionData> {
        let reader = File::open(path)?;
        Ok(serde_json::from_reader(reader)?)
    }

    fn save(&self, path: &PathBuf) -> Result {
        fs::create_dir_all(path.parent().ok_or(Error::NoNomeDirectory)?)?;
        let writer = File::create(path)?;
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    fn new(homeserver: Url, response: LoginResponse) -> Self {
        let LoginResponse {
            access_token,
            device_id,
            user_id,
            ..
        } = response;
        Self {
            homeserver,
            access_token,
            device_id,
            user_id,
        }
    }
}

impl From<SessionData> for Session {
    fn from(session: SessionData) -> Self {
        Self {
            access_token: session.access_token,
            device_id: session.device_id,
            user_id: session.user_id,
        }
    }
}

pub(crate) struct MatrixClient {
    client: Client,
    session_file: PathBuf,
}

impl MatrixClient {
    fn new(client: Client, dirs: &Directories) -> Self {
        Self {
            client,
            session_file: dirs.session_file.clone(),
        }
    }

    fn create_client(homserver: Url) -> Result<Client> {
        let client_config = ClientConfig::default();
        Ok(Client::new_with_config(homserver, client_config)?)
    }

    pub(crate) async fn load(dirs: &Directories) -> Result<Self> {
        if dirs.session_file.exists() {
            let session = SessionData::load(&dirs.session_file)?;

            let client = Self::create_client(session.homeserver.clone())?;
            client.restore_login(session.into()).await?;

            let client = Self::new(client, dirs);
            client.sync_once().await?;
            Ok(client)
        } else {
            Err(Error::NotLoggedIn)
        }
    }

    pub(crate) async fn login(
        dirs: &Directories,
        homeserver: &Url,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let client = Self::create_client(homeserver.clone())?;
        SessionData::new(
            homeserver.clone(),
            client
                .login(username, password, None, Some(crate::APP_NAME))
                .await?,
        )
        .save(&dirs.session_file)?;

        let client = Self::new(client, dirs);
        client.sync_once().await?;
        Ok(client)
    }

    pub(crate) async fn logout(self) -> Result {
        let Self {
            client,
            session_file,
        } = self;

        // TODO: send logout to server
        let _ = client;

        fs::remove_file(session_file)?;
        Ok(())
    }

    pub(crate) async fn sync_once(&self) -> Result {
        self.client.sync_once(SyncSettings::new()).await?;
        Ok(())
    }

    pub(crate) async fn join_room(
        &self,
        room: &RoomIdOrAliasId,
        servers: &[Box<ServerName>],
    ) -> Result {
        self.client.join_room_by_id_or_alias(room, servers).await?;
        Ok(())
    }

    pub(crate) async fn leave_room(&self, room: &RoomId) -> Result {
        self.client.leave_room(room).await?;
        Ok(())
    }

    pub(crate) async fn joined_rooms(&self) -> Vec<Arc<RwLock<Room>>> {
        self.client
            .joined_rooms()
            .read()
            .await
            .iter()
            .map(|i| i.1.clone())
            .collect()
    }

    pub(crate) async fn invited_rooms(&self) -> Vec<Arc<RwLock<Room>>> {
        self.client
            .invited_rooms()
            .read()
            .await
            .iter()
            .map(|i| i.1.clone())
            .collect()
    }

    pub(crate) async fn left_rooms(&self) -> Vec<Arc<RwLock<Room>>> {
        self.client
            .left_rooms()
            .read()
            .await
            .iter()
            .map(|i| i.1.clone())
            .collect()
    }

    pub(crate) async fn send(&self, room: &RoomId, message: TextMessageEventContent) -> Result {
        self.client
            .room_send(
                room,
                AnyMessageEventContent::RoomMessage(MessageEventContent::Text(message)),
                None,
            )
            .await?;
        Ok(())
    }
}
