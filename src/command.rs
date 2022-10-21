use std::io::{self, Write};

use crate::{dir::Directories, matrix::MatrixClient, Error, Result};

use url::Url;

use clap::Parser;

mod loggedin;

#[derive(Debug, Parser)]
pub(crate) enum Command {
    #[clap(flatten)]
    LoggedInCommands(loggedin::Command),

    /// Login to Matrix Account
    Login(LoginCommand),

    /// Emoji verify device of Matrix Account
    Verify(VerifyCommand),

    /// Logout from Matrix Account
    Logout(LogoutCommand),
}

impl Command {
    pub(super) async fn run(self, client: Result<MatrixClient>, dirs: &Directories) -> Result {
        match self {
            Self::Login(command) => command.run(client, dirs).await,
            Self::Verify(command) => command.run(client, dirs).await,
            Self::Logout(command) => command.run(client, dirs).await,
            Self::LoggedInCommands(command) => {
                let client = client?;
                command.run(client).await
            }
        }
    }
}

#[derive(Debug, Parser)]
pub(crate) struct LoginCommand {
    /// Homeserver Url
    homeserver: Url,

    /// Matrix Account Username
    username: Option<String>,

    /// Matrix Account Password
    password: Option<String>,
}

impl LoginCommand {
    async fn run(self, client: Result<MatrixClient>, dirs: &Directories) -> Result {
        if client.is_ok() {
            Error::custom("Already logged in")
        } else {
            let username = self
                .username
                .map_or_else(|| Self::user_input("Username:"), Ok)?;
            let password = self
                .password
                .map_or_else(|| Self::user_input("Password:"), Ok)?;
            MatrixClient::login(dirs, &self.homeserver, username.trim(), password.trim()).await?;
            Ok(())
        }
    }

    fn user_input(message: &'static str) -> Result<String> {
        print!("{} ", message);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line)
    }
}

#[derive(Debug, Parser)]
pub(crate) struct VerifyCommand {}

impl VerifyCommand {
    async fn run(self, client: Result<MatrixClient>, _dirs: &Directories) -> Result {
        if let Ok(client) = client {
            client.verify().await?;
            Ok(())
        } else {
            Error::custom("Not logged in")
        }
    }
}

#[derive(Debug, Parser)]
pub(crate) struct LogoutCommand {}

impl LogoutCommand {
    async fn run(self, client: Result<MatrixClient>, dirs: &Directories) -> Result {
        MatrixClient::logout(client, dirs).await?;
        Ok(())
    }
}
