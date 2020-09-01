use crate::{dir::Directories, matrix::MatrixClient, Error, Result};

use url::Url;

use structopt::StructOpt;

mod loggedin;

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    #[structopt(flatten)]
    LoggedInCommand(loggedin::Command),
    Login(LoginCommand),
}

impl Command {
    pub(super) async fn run(self, client: Option<MatrixClient>, dirs: &Directories) -> Result {
        match self {
            Self::Login(command) => command.run(client, dirs).await,
            Self::LoggedInCommand(command) => {
                if let Some(client) = client {
                    command.run(client).await
                } else {
                    Error::custom("Not logged in")
                }
            }
        }
    }
}

#[derive(Debug, StructOpt)]
pub(crate) struct LoginCommand {
    homeserver: Url,
    username: Option<String>,
    password: Option<String>,
}

impl LoginCommand {
    async fn run(self, client: Option<MatrixClient>, dirs: &Directories) -> Result {
        if let Some(_) = client {
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
