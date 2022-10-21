use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;

use crate::dir::Directories;
use crate::{Error, Result};

use matrix_sdk::config::StoreConfig;
use matrix_sdk::{
    config::SyncSettings,
    room,
    ruma::{OwnedDeviceId, OwnedUserId, RoomId},
    Client, Session,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use url::Url;

#[derive(Serialize, Deserialize)]
struct SessionData {
    homeserver: Url,
    access_token: String,
    device_id: OwnedDeviceId,
    user_id: OwnedUserId,
    refresh_token: Option<String>,
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

    fn new(
        homeserver: Url,
        access_token: String,
        device_id: OwnedDeviceId,
        user_id: OwnedUserId,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            homeserver,
            access_token,
            device_id,
            user_id,
            refresh_token,
        }
    }
}

impl From<SessionData> for Session {
    fn from(session: SessionData) -> Self {
        Self {
            access_token: session.access_token,
            device_id: session.device_id,
            user_id: session.user_id,
            refresh_token: session.refresh_token,
        }
    }
}

pub(crate) struct MatrixClient {
    client: Client,
    // dirs: Directories,  // not yet used
}

impl Deref for MatrixClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl MatrixClient {
    fn new(client: Client, _dirs: &Directories) -> Self {
        Self {
            client,
            // dirs: dirs.clone(),  // not yet used
        }
    }

    async fn create_client(homeserver: Url, dirs: &Directories) -> Result<Client> {
        // The location to save files to
        let sledhome = &dirs.sled_store_dir;
        info!("Using sled store {:?}", &sledhome);
        // let builder = if let Some(proxy) = cli.proxy { builder.proxy(proxy) } else { builder };
        let builder = Client::builder()
            .homeserver_url(homeserver)
            .store_config(StoreConfig::new());
        let client = builder
            .sled_store(&sledhome, None)
            .expect("Cannot add sled store to ClientBuilder.")
            .build()
            .await
            .expect("ClientBuilder build failed."); // no password for sled!
        Ok(client)
    }

    pub(crate) async fn load(dirs: &Directories) -> Result<Self> {
        if dirs.session_file.exists() {
            let session = SessionData::load(&dirs.session_file)?;

            let client = Self::create_client(session.homeserver.clone(), dirs).await?;
            info!("restored this session device_id = {:?}", &session.device_id);
            client.restore_login(session.into()).await?;
            let client = Self::new(client, dirs);
            info!("syncing ...");
            client.sync_once().await?;
            info!("sync completed");
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
    ) -> Result {
        let client = Self::create_client(homeserver.clone(), dirs).await?;
        _ = client
            .login_username(&username, password)
            .initial_device_display_name(crate::APP_NAME)
            .send()
            .await;

        let session = client.session().expect("Client should be logged in");
        info!("device id = {}", session.device_id);
        info!("session file = {:?}", dirs.session_file);

        SessionData::new(
            homeserver.clone(),
            session.access_token.clone(),
            session.device_id.clone(),
            session.user_id.clone(),
            session.refresh_token.clone(),
        )
        .save(&dirs.session_file)?;
        info!("new session file created = {:?}", dirs.session_file);

        client.sync_once(SyncSettings::new()).await?; // login
        Ok(())
    }

    pub(crate) async fn verify(self) -> Result {
        let Self { client, .. } = self;
        info!("Client logged in: {}", client.logged_in());
        info!("Client access token used: {:?}", client.access_token());
        sync(client).await?; // wait in sync for other party to initiate emoji verify
        Ok(())
    }

    pub(crate) async fn logout(client: Result<MatrixClient>, dirs: &Directories) -> Result {
        if let Ok(client) = client {
            // is logged in
            client.logout_server().await?;
        }
        match fs::remove_file(&dirs.session_file) {
            Ok(()) => info!("Session file successfully remove {:?}", &dirs.session_file),
            Err(e) => error!(
                "Error: Session file not removed. {:?} {:?}",
                &dirs.session_file, e
            ),
        }
        match fs::remove_dir_all(&dirs.sled_store_dir) {
            Ok(()) => info!(
                "Sled directory successfully remove {:?}",
                &dirs.sled_store_dir
            ),
            Err(e) => error!(
                "Error: Sled directory not removed. {:?} {:?}",
                &dirs.sled_store_dir, e
            ),
        }
        Ok(())
    }

    pub(crate) async fn logout_server(self) -> Result {
        match self.client.logout().await {
            Ok(n) => info!("Logout sent to server {:?}", n),
            Err(e) => error!(
                "Error: Server logout failed but we remove local device id anyway. {:?}",
                e
            ),
        }
        Ok(())
    }

    pub(crate) async fn sync_once(&self) -> Result {
        self.client.sync_once(SyncSettings::new()).await?;
        Ok(())
    }

    /*pub(crate) fn room(&self, room_id: &RoomId) -> Result<room::Room> {
        self.get_room(room_id).ok_or(Error::InvalidRoom)
    }*/

    pub(crate) fn joined_room(&self, room_id: &RoomId) -> Result<room::Joined> {
        self.get_joined_room(room_id).ok_or(Error::InvalidRoom)
    }

    /*pub(crate) fn invited_room(&self, room_id: &RoomId) -> Result<room::Invited> {
        self.get_invited_room(room_id).ok_or(Error::InvalidRoom)
    }*/

    /*pub(crate) fn left_room(&self, room_id: &RoomId) -> Result<room::Left> {
        self.get_left_room(room_id).ok_or(Error::InvalidRoom)
    }*/
}

// Code for emoji verify
use matrix_sdk::{
    self,
    encryption::verification::{format_emojis, SasVerification, Verification},
    ruma::{
        events::{
            key::verification::{
                done::{OriginalSyncKeyVerificationDoneEvent, ToDeviceKeyVerificationDoneEvent},
                key::{OriginalSyncKeyVerificationKeyEvent, ToDeviceKeyVerificationKeyEvent},
                request::ToDeviceKeyVerificationRequestEvent,
                start::{OriginalSyncKeyVerificationStartEvent, ToDeviceKeyVerificationStartEvent},
            },
            room::message::{MessageType, OriginalSyncRoomMessageEvent},
        },
        UserId,
    },
};
use std::io::{self, Write};

async fn wait_for_confirmation(client: Client, sas: SasVerification) {
    let emoji = sas.emoji().expect("The emojis should be available now.");

    println!("\nDo the emojis match: \n{}", format_emojis(emoji));
    print!("Confirm with `yes` or cancel with `no` or Control-C to abort: ");
    std::io::stdout()
        .flush()
        .expect("We should be able to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("error: unable to read user input");

    match input.trim().to_lowercase().as_ref() {
        "yes" | "y" | "true" | "ok" => {
            info!("Received 'Yes'!");
            sas.confirm().await.unwrap();

            if sas.is_done() {
                print_devices(sas.other_device().user_id(), &client).await;
                print_result(&sas);
            } else {
                info!("Sas not done yet.");
            }
        }
        _ => {
            info!("Cancelling. Sorry!");
            sas.cancel().await.unwrap();
        }
    }
}

fn print_result(sas: &SasVerification) {
    let device = sas.other_device();

    println!(
        "Successfully verified device {} {} {:?}",
        device.user_id(),
        device.device_id(),
        device.local_trust_state()
    );

    println!("\nDo more Emoji verifications or hit Control-C to terminate program.\n");
}

async fn print_devices(user_id: &UserId, client: &Client) {
    info!("Devices of user {}", user_id);

    for device in client
        .encryption()
        .get_user_devices(user_id)
        .await
        .unwrap()
        .devices()
    {
        info!(
            "   {:<10} {:<30} {:<}",
            device.device_id(),
            device.display_name().unwrap_or("-"),
            device.is_verified()
        );
    }
}

async fn sync(client: Client) -> matrix_sdk::Result<()> {
    client.add_event_handler(
        |ev: ToDeviceKeyVerificationRequestEvent, client: Client| async move {
            info!("ToDeviceKeyVerificationRequestEvent");
            let request = client
                .encryption()
                .get_verification_request(&ev.sender, &ev.content.transaction_id)
                .await
                .expect("Request object wasn't created");

            request
                .accept()
                .await
                .expect("Can't accept verification request");
        },
    );

    client.add_event_handler(
        |ev: ToDeviceKeyVerificationStartEvent, client: Client| async move {
            info!("ToDeviceKeyVerificationStartEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                .await
            {
                info!(
                    "Starting verification with {} {}",
                    &sas.other_device().user_id(),
                    &sas.other_device().device_id()
                );
                print_devices(&ev.sender, &client).await;
                sas.accept().await.unwrap();
            }
        },
    );

    client.add_event_handler(
        |ev: ToDeviceKeyVerificationKeyEvent, client: Client| async move {
            info!("ToDeviceKeyVerificationKeyEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                .await
            {
                tokio::spawn(wait_for_confirmation(client, sas));
            }
        },
    );

    client.add_event_handler(
        |ev: ToDeviceKeyVerificationDoneEvent, client: Client| async move {
            info!("ToDeviceKeyVerificationDoneEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                .await
            {
                if sas.is_done() {
                    print_result(&sas);
                    print_devices(&ev.sender, &client).await;
                }
            }
        },
    );

    client.add_event_handler(
        |ev: OriginalSyncRoomMessageEvent, client: Client| async move {
            info!("OriginalSyncRoomMessageEvent");
            if let MessageType::VerificationRequest(_) = &ev.content.msgtype {
                let request = client
                    .encryption()
                    .get_verification_request(&ev.sender, &ev.event_id)
                    .await
                    .expect("Request object wasn't created");

                request
                    .accept()
                    .await
                    .expect("Can't accept verification request");
            }
        },
    );

    client.add_event_handler(
        |ev: OriginalSyncKeyVerificationStartEvent, client: Client| async move {
            info!("OriginalSyncKeyVerificationStartEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                .await
            {
                println!(
                    "Starting verification with {} {}",
                    &sas.other_device().user_id(),
                    &sas.other_device().device_id()
                );
                print_devices(&ev.sender, &client).await;
                sas.accept().await.unwrap();
            }
        },
    );

    client.add_event_handler(
        |ev: OriginalSyncKeyVerificationKeyEvent, client: Client| async move {
            info!("OriginalSyncKeyVerificationKeyEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                .await
            {
                tokio::spawn(wait_for_confirmation(client.clone(), sas));
            }
        },
    );

    client.add_event_handler(
        |ev: OriginalSyncKeyVerificationDoneEvent, client: Client| async move {
            info!("OriginalSyncKeyVerificationDoneEvent");
            if let Some(Verification::SasV1(sas)) = client
                .encryption()
                .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                .await
            {
                if sas.is_done() {
                    print_result(&sas);
                    print_devices(&ev.sender, &client).await;
                }
            }
        },
    );

    // go into event loop to sync and to execute verify protocol
    println!("Ready and waiting ...");
    println!("Go to other Matrix client like Element and initiate Emoji verification there.");
    println!("Best to have the other Matrix client ready and waiting before you start");
    println!("{}.", crate::APP_NAME);
    client.sync(SyncSettings::new()).await?;

    Ok(())
}
