use crate::lockfile::ToolEntry as LockfileToolEntry;
use crate::{ui, Lockfile, Workspace, WorkspacePath};
use anyhow::Result;
use chrono::{DateTime, Local};
use directories::BaseDirs;
use std::collections::{BTreeSet, HashMap};
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

    report_tools(workspace, lockfile.as_ref(), &display)?;

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

fn report_tools(
    workspace: &Workspace,
    lockfile: Option<&Lockfile>,
    display: &DisplayContext,
) -> Result<()> {
    let tools = workspace.tools()?;

    let mut recorded: HashMap<String, Vec<&LockfileToolEntry>> = HashMap::new();
    if let Some(lockfile) = lockfile {
        for entry in lockfile.tool_symlinks() {
            recorded.entry(entry.name.clone()).or_default().push(entry);
        }
    }

    if tools.is_empty() {
        if recorded.is_empty() {
            ui::info("No tools defined for the active profile.");
        } else {
            ui::warn("Lockfile lists tools that are not defined in the active profile:");
            for (name, entries) in recorded {
                let targets = join_targets(&entries, display);
                ui::warn(format!(
                    "Tool '{}' remains installed (targets: {}). Run 'dws cleanup' to remove it.",
                    name, targets
                ));
            }
        }
        return Ok(());
    }

    let total_defined = tools.len();
    let mut installed_ok = 0usize;
    let mut reports: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();

    for (name, _entry) in tools.iter() {
        let entries = recorded.remove(name).unwrap_or_default();
        if entries.is_empty() {
            reports.push((
                name.clone(),
                Vec::new(),
                vec![format!(
                    "Tool '{}' is defined but not installed. Run 'dws sync' to install it.",
                    name
                )],
            ));
            continue;
        }

        let versions_set = collect_versions(&entries);
        let versions = if versions_set.is_empty() {
            vec!["unknown".to_string()]
        } else {
            versions_set.into_iter().collect::<Vec<_>>()
        };

        let mut issues = Vec::new();
        for entry in &entries {
            match check_symlink(&entry.source, &entry.target) {
                LinkState::Ok => {}
                LinkState::MissingTarget => issues.push(format!(
                    "Tool '{}' {} missing symlink at {} (expected source {}).",
                    name,
                    entry.version,
                    display.format(&entry.target),
                    display.format(&entry.source)
                )),
                LinkState::NotSymlink => issues.push(format!(
                    "Tool '{}' {} target exists but is not a symlink: {}.",
                    name,
                    entry.version,
                    display.format(&entry.target)
                )),
                LinkState::WrongTarget { actual } => issues.push(format!(
                    "Tool '{}' {} target points to {} (expected {}).",
                    name,
                    entry.version,
                    display.format(&actual),
                    display.format(&entry.source)
                )),
                LinkState::MissingSource => issues.push(format!(
                    "Tool '{}' {} source missing at {} (symlink at {}).",
                    name,
                    entry.version,
                    display.format(&entry.source),
                    display.format(&entry.target)
                )),
                LinkState::IoError(err) => issues.push(format!(
                    "Tool '{}' {}: failed to inspect {} ({})",
                    name,
                    entry.version,
                    display.format(&entry.target),
                    err
                )),
            }
        }

        if issues.is_empty() {
            installed_ok += 1;
        }

        reports.push((name.clone(), versions, issues));
    }

    ui::status(
        "Tools",
        format!("{}/{} installed", installed_ok, total_defined),
    );

    for (name, versions, issues) in reports {
        if issues.is_empty() {
            let version_display = versions.join(", ");
            ui::success("Tool", format!("{} {}", name, version_display));
        } else {
            for issue in issues {
                ui::warn(issue);
            }
        }
    }

    for (name, entries) in recorded {
        let versions = collect_versions(&entries);
        let version_display = if versions.is_empty() {
            "unknown".to_string()
        } else {
            versions.iter().cloned().collect::<Vec<_>>().join(", ")
        };
        let targets = join_targets(&entries, display);
        ui::warn(format!(
            "Tool '{}' {} is recorded in the lockfile but not defined in the active profile (targets: {}).",
            name, version_display, targets
        ));
    }

    Ok(())
}

fn collect_versions(entries: &[&LockfileToolEntry]) -> BTreeSet<String> {
    let mut versions = BTreeSet::new();
    for entry in entries {
        versions.insert(entry.version.clone());
    }
    versions
}

fn join_targets(entries: &[&LockfileToolEntry], display: &DisplayContext) -> String {
    entries
        .iter()
        .map(|entry| display.format(&entry.target))
        .collect::<Vec<_>>()
        .join(", ")
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
