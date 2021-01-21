use std::fs;

use crate::{dir::Directories, matrix::MatrixClient, Error, Result};

use url::Url;

use structopt::StructOpt;

mod loggedin;

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    #[structopt(flatten)]
    LoggedInCommand(loggedin::Command),
    /// Login to Matrix Account
    Login(LoginCommand),
    /// Logout from Matrix Account
    Logout(LogoutCommand),
}

impl Command {
    pub(super) async fn run(self, client: Result<MatrixClient>, dirs: &Directories) -> Result {
        match self {
            Self::Login(command) => command.run(client, dirs).await,
            Self::Logout(command) => command.run(client, dirs).await,
            Self::LoggedInCommand(command) => {
                let client = client?;
                command.run(client).await
            }
        }
    }
}

#[derive(Debug, StructOpt)]
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
        if let Ok(_) = client {
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
        println!("{}", message);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line)
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct LogoutCommand {}

impl LogoutCommand {
    async fn run(self, client: Result<MatrixClient>, dirs: &Directories) -> Result {
        if let Ok(client) = client {
            client.logout().await?;
        } else {
            if dirs.session_file.exists() {
                fs::remove_file(&dirs.session_file)?;
            }
            client?;
        }
        Ok(())
    }
}
