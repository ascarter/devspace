use crate::lockfile::ToolReceipt;
use crate::{ui, Lockfile, Workspace, WorkspacePath};
use anyhow::Result;
use chrono::{DateTime, Local};
use directories::BaseDirs;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn execute(workspace: &Workspace) -> Result<()> {
    if !workspace.exists() {
        ui::warn("Workspace not initialized. Run 'dws init' first.");
        return Ok(());
    }

    let profile_name = workspace.active_profile_name().to_string();
    let profile_path = workspace.path(WorkspacePath::Profile);
    let workspace_root = workspace.path(WorkspacePath::Root);
    let display = DisplayContext::new(workspace_root);

    ui::success(
        "Active",
        format!(
            "profile '{}' at {}",
            profile_name,
            display.format(&profile_path)
        ),
    );

    let lockfile_path = workspace.path(WorkspacePath::Lockfile);
    let lockfile = if lockfile_path.exists() {
        let loaded = Lockfile::load(&lockfile_path)?;
        let formatted = format_timestamp(&loaded.metadata.installed_at);
        ui::status("Last Sync", formatted);
        Some(loaded)
    } else {
        ui::info("Lockfile not found. Run 'dws sync' to install the workspace state.");
        None
    };

    if let Some(lockfile) = lockfile.as_ref() {
        report_config_symlinks(lockfile, &display)?;
    } else {
        ui::info("Skipping config inspection because no lockfile is present.");
    }

    report_tools(workspace, lockfile.as_ref())?;

    Ok(())
}

fn report_config_symlinks(lockfile: &Lockfile, display: &DisplayContext) -> Result<()> {
    let entries: Vec<_> = lockfile.config_symlinks().collect();
    if entries.is_empty() {
        ui::info("No config symlinks recorded in the lockfile.");
        return Ok(());
    }

    let total = entries.len();
    let mut issues = Vec::new();
    for entry in entries {
        match check_symlink(&entry.source, &entry.target) {
            LinkState::Ok => {}
            LinkState::MissingTarget => issues.push(format!(
                "Config target missing: {} (expected -> {})",
                display.format(&entry.target),
                display.format(&entry.source)
            )),
            LinkState::NotSymlink => issues.push(format!(
                "Config target exists but is not a symlink: {} (expected -> {})",
                display.format(&entry.target),
                display.format(&entry.source)
            )),
            LinkState::WrongTarget { actual } => issues.push(format!(
                "Config target points to {} but lockfile expects {} -> {}",
                display.format(&actual),
                display.format(&entry.target),
                display.format(&entry.source)
            )),
            LinkState::MissingSource => issues.push(format!(
                "Config source missing: {} (symlink at {})",
                display.format(&entry.source),
                display.format(&entry.target)
            )),
            LinkState::IoError(err) => issues.push(format!(
                "Failed to inspect config symlink {}: {}",
                display.format(&entry.target),
                err
            )),
        }
    }

    if issues.is_empty() {
        ui::success("Config", format!("{} item(s) healthy", total));
    } else {
        ui::warn(format!("Config check found {} issue(s)", issues.len()));
        for issue in issues {
            ui::warn(issue);
        }
    }

    Ok(())
}

fn report_tools(workspace: &Workspace, lockfile: Option<&Lockfile>) -> Result<()> {
    let defined = workspace.tools()?;

    // Map of tool name -> receipts
    let mut receipts: HashMap<String, Vec<&ToolReceipt>> = HashMap::new();
    if let Some(lf) = lockfile {
        for r in lf.tool_receipts() {
            receipts.entry(r.name.clone()).or_default().push(r);
        }
    }

    if defined.is_empty() {
        if receipts.is_empty() {
            ui::info("No tools defined for the active profile.");
        } else {
            ui::warn("Installed tools found in lockfile but not defined in manifest:");
            for (name, rs) in receipts {
                let versions: Vec<String> = rs.iter().map(|r| r.resolved_version.clone()).collect();
                ui::warn(format!(
                    "Tool '{}' (installed versions: {}) - consider removing with future cleanup.",
                    name,
                    versions.join(", ")
                ));
            }
        }
        return Ok(());
    }

    let mut ok_count = 0usize;
    for (name, _) in defined.iter() {
        match receipts.remove(name) {
            None => {
                ui::warn(format!(
                    "Tool '{}' defined but no receipt found. Run 'dws sync' to install.",
                    name
                ));
            }
            Some(rs) => {
                // For Phase 0 we only show the first resolved version
                let versions: Vec<String> = rs.iter().map(|r| r.resolved_version.clone()).collect();
                ui::success("Tool", format!("{} {}", name, versions.join(", ")));
                ok_count += 1;
            }
        }
    }

    ui::status("Tools", format!("{}/{} installed", ok_count, defined.len()));

    // Any remaining receipts are orphaned
    for (name, rs) in receipts {
        let versions: Vec<String> = rs.iter().map(|r| r.resolved_version.clone()).collect();
        ui::warn(format!(
            "Orphaned tool '{}' (versions: {}) not present in manifest.",
            name,
            versions.join(", ")
        ));
    }

    Ok(())
}

fn format_timestamp(raw: &str) -> String {
    match DateTime::parse_from_rfc3339(raw) {
        Ok(instant) => {
            let local = instant.with_timezone(&Local);
            format!(
                "{} (recorded as {})",
                local.format("%Y-%m-%d %H:%M:%S %Z"),
                raw
            )
        }
        Err(_) => raw.to_string(),
    }
}

fn check_symlink(source: &Path, target: &Path) -> LinkState {
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink() {
                return LinkState::NotSymlink;
            }
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return LinkState::MissingTarget;
            }
            return LinkState::IoError(err);
        }
    }

    match fs::read_link(target) {
        Ok(actual) => {
            let resolved = if actual.is_absolute() {
                actual
            } else if let Some(parent) = target.parent() {
                parent.join(actual)
            } else {
                actual
            };

            if resolved != source {
                return LinkState::WrongTarget { actual: resolved };
            }
        }
        Err(err) => return LinkState::IoError(err),
    }

    if !(source.exists() || source.symlink_metadata().is_ok()) {
        return LinkState::MissingSource;
    }

    LinkState::Ok
}

enum LinkState {
    Ok,
    MissingTarget,
    NotSymlink,
    WrongTarget { actual: PathBuf },
    MissingSource,
    IoError(std::io::Error),
}

struct DisplayContext {
    workspace_root: PathBuf,
    home_dir: Option<PathBuf>,
}

impl DisplayContext {
    fn new(workspace_root: PathBuf) -> Self {
        let home_dir = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        Self {
            workspace_root,
            home_dir,
        }
    }

    #[allow(dead_code)]
    fn format(&self, path: &Path) -> String {
        if let Some(home) = &self.home_dir {
            if let Ok(stripped) = path.strip_prefix(home) {
                if stripped.as_os_str().is_empty() {
                    return "~".to_string();
                }
                return format!("~/{}", stripped.display());
            }
        }

        if let Ok(stripped) = path.strip_prefix(&self.workspace_root) {
            if stripped.as_os_str().is_empty() {
                return self.workspace_root.display().to_string();
            }
            return format!(
                "{}{}{}",
                self.workspace_root.display(),
                std::path::MAIN_SEPARATOR,
                stripped.display()
            );
        }

        path.display().to_string()
    }
}
