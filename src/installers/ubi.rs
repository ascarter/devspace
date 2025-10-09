use super::{sanitize_component, InstallContext, ToolInstaller};
use crate::lockfile::Lockfile;
use crate::manifest::ManifestEntry;
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use ubi::UbiBuilder;
use walkdir::WalkDir;

pub(crate) struct UbiInstaller {
    entry: ManifestEntry,
    install_root: PathBuf,
    version_label: String,
    bin_dir: PathBuf,
}

impl UbiInstaller {
    pub(crate) fn new(mut entry: ManifestEntry, context: InstallContext) -> Result<Self> {
        if entry.definition.project.is_none() {
            bail!(
                "Tool '{}' must specify `project` when using the ubi installer",
                entry.name
            );
        }

        if entry.definition.bin.is_empty() {
            let project = entry.definition.project.as_ref().expect("validated above");
            let default_bin = default_bin_name(project);
            entry.definition.bin = vec![default_bin];
        }

        let version_label = entry
            .definition
            .version
            .clone()
            .unwrap_or_else(|| "latest".to_string());

        let install_root = context
            .cache_apps_dir
            .join(sanitize_component(&entry.name))
            .join(sanitize_component(&version_label));

        Ok(Self {
            bin_dir: context.bin_dir,
            entry,
            install_root,
            version_label,
        })
    }

    fn install_release(&self, runtime: &mut Runtime) -> Result<()> {
        fs::create_dir_all(&self.install_root).with_context(|| {
            format!(
                "Failed to create ubi install directory {:?} for '{}'",
                self.install_root, self.entry.name
            )
        })?;

        let project = self
            .entry
            .definition
            .project
            .as_ref()
            .expect("validated above");

        let mut builder = UbiBuilder::new()
            .project(project)
            .install_dir(&self.install_root)
            .extract_all();

        if let Some(tag) = self.entry.definition.version.as_deref() {
            builder = builder.tag(tag);
        }

        let mut ubi = builder
            .build()
            .with_context(|| format!("Failed to configure ubi for '{}'", self.entry.name))?;

        // Despite the name, `install_binary` unpacks the entire release when `extract_all` is set.
        // We rely on that to preserve additional assets (man pages, completions, etc.) alongside
        // the primary binary and then wire up symlinks ourselves.
        runtime.block_on(async {
            ubi.install_binary()
                .await
                .with_context(|| format!("Failed to install '{}' via ubi", self.entry.name))
        })
    }

    fn binaries(&self) -> Result<Vec<(String, PathBuf)>> {
        let mut results = Vec::new();
        for bin_name in &self.entry.definition.bin {
            let source = self.find_binary_path(bin_name)?;
            results.push((bin_name.clone(), source));
        }
        Ok(results)
    }

    fn find_binary_path(&self, bin_name: &str) -> Result<PathBuf> {
        let direct = self.install_root.join(bin_name);
        if direct.exists() {
            return Ok(direct);
        }

        #[cfg(target_os = "windows")]
        let alt = self.install_root.join(format!("{bin_name}.exe"));
        #[cfg(not(target_os = "windows"))]
        let alt = self.install_root.join(format!("{bin_name}.exe"));
        if alt.exists() {
            return Ok(alt);
        }

        let mut matches = Vec::new();
        for entry in WalkDir::new(&self.install_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let name = entry.file_name().to_string_lossy();
            if name == bin_name || name == format!("{bin_name}.exe") {
                matches.push(entry.into_path());
            }
        }

        matches.into_iter().next().ok_or_else(|| {
            anyhow!(
                "Could not locate binary '{}' within {:?}",
                bin_name,
                self.install_root
            )
        })
    }

    fn link_binaries(
        &self,
        binaries: Vec<(String, PathBuf)>,
        lockfile: &mut Lockfile,
    ) -> Result<()> {
        fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("Failed to create bin directory {:?}", self.bin_dir))?;

        for (bin_name, source) in binaries {
            let target = self.bin_dir.join(&bin_name);
            if target.exists() || target.symlink_metadata().is_ok() {
                fs::remove_file(&target).with_context(|| {
                    format!("Failed to remove existing binary symlink {:?}", target)
                })?;
            }

            symlink(&source, &target).with_context(|| {
                format!("Failed to create symlink {:?} -> {:?}", target, source)
            })?;

            lockfile.add_tool_symlink(
                self.entry.name.clone(),
                self.version_label.clone(),
                source,
                target,
            );
        }

        Ok(())
    }
}

impl ToolInstaller for UbiInstaller {
    fn requires_runtime(&self) -> bool {
        true
    }

    fn install(&self, runtime: Option<&mut Runtime>, lockfile: &mut Lockfile) -> Result<()> {
        let runtime = runtime.ok_or_else(|| anyhow!("UBI installer requires a Tokio runtime"))?;
        self.install_release(runtime)?;
        let binaries = self.binaries()?;
        self.link_binaries(binaries, lockfile)
    }
}

// Tests: there isn't currently an automated test for this installer because the upstream `ubi`
// crate always talks to live forge endpoints. Once we introduce a mock downloader (for example via
// a local HTTP server that serves a tiny archive), we can exercise this logic end-to-end without
// hitting the network.

fn default_bin_name(project: &str) -> String {
    let mut candidate = project.trim_end_matches('/');
    if let Some(pos) = candidate.rfind('/') {
        candidate = &candidate[pos + 1..];
    }
    let candidate = candidate.trim_end_matches(".git");
    if candidate.is_empty() {
        "tool".to_string()
    } else {
        candidate.to_string()
    }
}
