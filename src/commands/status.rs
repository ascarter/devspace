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

    report_tools(workspace, &display, lockfile.as_ref())?;

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
    display: &DisplayContext,
    lockfile: Option<&Lockfile>,
) -> Result<()> {
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
                let versions: Vec<String> = rs.iter().map(|r| r.resolved_version.clone()).collect();
                let (binary_total, extra_total, asset_state, issues) =
                    verify_tool_receipts(&rs, display);

                let asset_summary = match asset_state {
                    AssetState::NotRecorded => "asset: none",
                    AssetState::Healthy => "asset: cached",
                    AssetState::Missing => "asset: missing",
                };

                if issues.is_empty() {
                    ui::success(
                        "Tool",
                        format!(
                            "{} {} (binaries: {}, extras: {}, {})",
                            name,
                            versions.join(", "),
                            binary_total,
                            extra_total,
                            asset_summary
                        ),
                    );
                    ok_count += 1;
                } else {
                    ui::warn(format!(
                        "Tool '{}' has {} issue(s) across version(s: {}) ({})",
                        name,
                        issues.len(),
                        versions.join(", "),
                        asset_summary
                    ));
                    for issue in issues {
                        ui::warn(issue);
                    }
                }
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

fn verify_tool_receipts(
    receipts: &[&ToolReceipt],
    display: &DisplayContext,
) -> (usize, usize, AssetState, Vec<String>) {
    let mut issues = Vec::new();
    let mut binaries = 0usize;
    let mut extras = 0usize;
    let mut asset_recorded = false;
    let mut asset_missing = false;

    for receipt in receipts {
        for bin in &receipt.binaries {
            binaries += 1;
            match check_symlink(&bin.source, &bin.target) {
                LinkState::Ok => {}
                LinkState::MissingTarget => issues.push(format!(
                    "Binary target missing: {} (expected -> {})",
                    display.format(&bin.target),
                    display.format(&bin.source)
                )),
                LinkState::NotSymlink => issues.push(format!(
                    "Binary target exists but is not a symlink: {}",
                    display.format(&bin.target)
                )),
                LinkState::WrongTarget { actual } => issues.push(format!(
                    "Binary target points to {} but lockfile expects {} -> {}",
                    display.format(&actual),
                    display.format(&bin.target),
                    display.format(&bin.source)
                )),
                LinkState::MissingSource => issues.push(format!(
                    "Binary source missing: {} (symlink at {})",
                    display.format(&bin.source),
                    display.format(&bin.target)
                )),
                LinkState::IoError(err) => issues.push(format!(
                    "Failed to inspect binary symlink {}: {}",
                    display.format(&bin.target),
                    err
                )),
            }
        }

        for extra in &receipt.extras {
            extras += 1;
            match check_symlink(&extra.source, &extra.target) {
                LinkState::Ok => {}
                LinkState::MissingTarget => issues.push(format!(
                    "Extra target missing ({}): {}",
                    extra.kind,
                    display.format(&extra.target)
                )),
                LinkState::NotSymlink => issues.push(format!(
                    "Extra target exists but is not a symlink ({}): {}",
                    extra.kind,
                    display.format(&extra.target)
                )),
                LinkState::WrongTarget { actual } => issues.push(format!(
                    "Extra symlink mismatch ({}): {} points to {} but expected {}",
                    extra.kind,
                    display.format(&extra.target),
                    display.format(&actual),
                    display.format(&extra.source)
                )),
                LinkState::MissingSource => issues.push(format!(
                    "Extra source missing ({}): {}",
                    extra.kind,
                    display.format(&extra.source)
                )),
                LinkState::IoError(err) => issues.push(format!(
                    "Failed to inspect extra symlink {} ({}): {}",
                    display.format(&extra.target),
                    extra.kind,
                    err
                )),
            }
        }

        if let Some(asset) = &receipt.asset {
            asset_recorded = true;
            if !asset.archive_path.exists() {
                issues.push(format!(
                    "Asset archive missing: {}",
                    display.format(&asset.archive_path)
                ));
                asset_missing = true;
            }
            if !asset.extract_dir.exists() {
                issues.push(format!(
                    "Asset contents directory missing: {}",
                    display.format(&asset.extract_dir)
                ));
                asset_missing = true;
            }
        }
    }

    let asset_state = if asset_recorded {
        if asset_missing {
            AssetState::Missing
        } else {
            AssetState::Healthy
        }
    } else {
        AssetState::NotRecorded
    };

    (binaries, extras, asset_state, issues)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetState {
    NotRecorded,
    Healthy,
    Missing,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::AssetRecord;
    use tempfile::TempDir;

    fn display_context() -> DisplayContext {
        DisplayContext::new(PathBuf::from("/workspace"))
    }

    #[test]
    fn verify_tool_receipts_reports_missing_asset() {
        let receipt = ToolReceipt {
            name: "mock".to_string(),
            manifest_version: "latest".to_string(),
            resolved_version: "v1.0.0".to_string(),
            installer_kind: "github".to_string(),
            installed_at: "2025-01-01T00:00:00Z".to_string(),
            binaries: Vec::new(),
            extras: Vec::new(),
            asset: Some(AssetRecord {
                name: "mock.tar.gz".to_string(),
                url: "https://example.com/mock.tar.gz".to_string(),
                checksum: "deadbeef".to_string(),
                archive_path: PathBuf::from("/tmp/missing.tar.gz"),
                extract_dir: PathBuf::from("/tmp/missing"),
                pattern_index: Some(0),
                pattern: Some("mock".to_string()),
            }),
        };

        let (_b, _e, asset_state, issues) = verify_tool_receipts(&[&receipt], &display_context());
        assert_eq!(asset_state, AssetState::Missing);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("Asset archive missing")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("Asset contents directory missing")));
    }

    #[test]
    fn verify_tool_receipts_reports_healthy_asset() {
        let temp = TempDir::new().unwrap();
        let archive_path = temp.path().join("mock.tar.gz");
        let extract_dir = temp.path().join("contents");
        std::fs::write(&archive_path, b"tar").unwrap();
        std::fs::create_dir_all(&extract_dir).unwrap();

        let receipt = ToolReceipt {
            name: "mock".to_string(),
            manifest_version: "latest".to_string(),
            resolved_version: "v1.0.0".to_string(),
            installer_kind: "github".to_string(),
            installed_at: "2025-01-01T00:00:00Z".to_string(),
            binaries: Vec::new(),
            extras: Vec::new(),
            asset: Some(AssetRecord {
                name: "mock.tar.gz".to_string(),
                url: "https://example.com/mock.tar.gz".to_string(),
                checksum: "deadbeef".to_string(),
                archive_path,
                extract_dir,
                pattern_index: Some(0),
                pattern: Some("mock".to_string()),
            }),
        };

        let (_b, _e, asset_state, issues) = verify_tool_receipts(&[&receipt], &display_context());
        assert_eq!(asset_state, AssetState::Healthy);
        assert!(issues.is_empty());
    }

    #[test]
    fn verify_tool_receipts_reports_not_recorded() {
        let receipt = ToolReceipt {
            name: "mock".to_string(),
            manifest_version: "latest".to_string(),
            resolved_version: "v1.0.0".to_string(),
            installer_kind: "github".to_string(),
            installed_at: "2025-01-01T00:00:00Z".to_string(),
            binaries: Vec::new(),
            extras: Vec::new(),
            asset: None,
        };

        let (_b, _e, asset_state, issues) = verify_tool_receipts(&[&receipt], &display_context());
        assert_eq!(asset_state, AssetState::NotRecorded);
        assert!(issues.is_empty());
    }
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
