use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::dir::Directories;
use crate::{Error, Result};

use matrix_sdk::{
    ruma::api::client::r0::session::login::Response as LoginResponse,
    ruma::events::room::message::{MessageEventContent, MessageType, TextMessageEventContent},
    ruma::identifiers::{DeviceId, RoomId, RoomIdOrAliasId, ServerName, UserId},
    Client, ClientConfig, Session, SyncSettings,
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
    fn load(path: &Path) -> Result<SessionData> {
        let reader = File::open(path)?;
        // matrix-send-rs used to create session.js as world-readable, so just ensuring the correct
        // permissions during writing isn't good enough. We also need to fix the existing files.
        SessionData::set_permissions(&reader)?;
        Ok(serde_json::from_reader(reader)?)
    }

    fn save(&self, path: &Path) -> Result {
        fs::create_dir_all(path.parent().ok_or(Error::NoNomeDirectory)?)?;
        let writer = File::create(path)?;
        serde_json::to_writer_pretty(&writer, self)?;
        SessionData::set_permissions(&writer)?;
        Ok(())
    }

    #[cfg(unix)]
    fn set_permissions(file: &File) -> Result {
        use std::os::unix::fs::PermissionsExt;

        let perms = file.metadata()?.permissions();

        // is the file world-readable? if so, reset the permissions to 600
        if perms.mode() & 0o4 == 0o4 {
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn set_permissions(file: &File) -> Result {
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

impl Deref for MatrixClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
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

    pub(crate) async fn send(&self, room: &RoomId, message: TextMessageEventContent) -> Result {
        self.client
            .room_send(
                room,
                MessageEventContent::new(MessageType::Text(message)),
                None,
            )
            .await?;
        Ok(())
    }
}
