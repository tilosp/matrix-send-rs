use std::fs;
use std::path::PathBuf;

use crate::{Error, Result};

use directories::ProjectDirs;

use tracing::info;

const SESSION_FILE: &str = "session.json";
const SLED_STORE_DIR: &str = "sledstore";

#[derive(Clone)]
pub(crate) struct Directories {
    pub(crate) session_file: PathBuf,
    pub(crate) sled_store_dir: PathBuf,
}

impl Directories {
    pub(crate) fn new() -> Result<Self> {
        let dirs =
            ProjectDirs::from_path(PathBuf::from(crate::APP_NAME)).ok_or(Error::NoNomeDirectory)?;

        fs::create_dir_all(dirs.data_dir())?;
        let sf = dirs.data_dir().join(SESSION_FILE);
        let ssd = dirs.data_dir().join(SLED_STORE_DIR);
        info!(
            "Data will be put into project directory {}.",
            dirs.data_dir().display()
        );
        info!("Session file with access token is {}.", sf.display());
        info!("Sled store directory for encryption is {}.", ssd.display());
        Ok(Directories {
            session_file: sf,
            sled_store_dir: ssd,
        })
    }
}
