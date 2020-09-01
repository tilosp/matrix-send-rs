use std::fs;
use std::path::PathBuf;

use crate::{Error, Result};

use directories::ProjectDirs;

const SESSION_FILE: &str = "session.json";

pub(crate) struct Directories {
    pub(crate) session_file: PathBuf,
}

impl Directories {
    pub(crate) fn new() -> Result<Self> {
        let dirs =
            ProjectDirs::from_path(PathBuf::from(crate::APP_NAME)).ok_or(Error::NoNomeDirectory)?;

        fs::create_dir_all(dirs.data_dir())?;
        Ok(Directories {
            session_file: dirs.data_dir().join(SESSION_FILE),
        })
    }
}
