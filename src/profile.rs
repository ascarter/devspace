use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Profile {
    root: PathBuf,
}

impl Profile {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("manifests")
    }
}
