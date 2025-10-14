use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Profile {
    name: String,
    root: PathBuf,
}

impl Profile {
    pub fn new(name: impl Into<String>, root: PathBuf) -> Self {
        Self {
            name: name.into(),
            root,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("dws.toml")
    }
}
