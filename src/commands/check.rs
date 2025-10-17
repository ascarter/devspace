use crate::toolset::{validate_tool_config, ManifestIssue, ToolConfigFile};
use crate::workspace::WorkspacePath;
use crate::{ui, Workspace};
use anyhow::Result;
use std::collections::HashSet;
use std::fs;

pub fn execute(workspace: &Workspace) -> Result<()> {
    let mut issues = Vec::new();
    let mut validated = 0usize;
    let mut visited = HashSet::new();

    // Always validate the active profile first so errors surface prominently.
    let active_manifest = workspace.path(WorkspacePath::ProfileConfig);
    if active_manifest.exists() {
        let config = ToolConfigFile::load(&active_manifest)?;
        issues.extend(validate_tool_config(&active_manifest, &config));
        validated += 1;
    } else {
        issues.push(ManifestIssue::general(
            &active_manifest,
            format!(
                "active profile '{}' is missing dws.toml",
                workspace.active_profile_name()
            ),
        ));
    }
    visited.insert(active_manifest.clone());

    // Validate every profile on disk (deduplicating the active profile).
    let profiles_dir = workspace.path(WorkspacePath::Profiles);
    if profiles_dir.exists() {
        for entry in fs::read_dir(&profiles_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("dws.toml");
            if !visited.insert(manifest_path.clone()) {
                continue;
            }

            if !manifest_path.exists() {
                let profile_name = entry.file_name().to_string_lossy().into_owned();
                issues.push(ManifestIssue::general(
                    &manifest_path,
                    format!("profile '{profile_name}' is missing dws.toml"),
                ));
                continue;
            }

            let config = ToolConfigFile::load(&manifest_path)?;
            issues.extend(validate_tool_config(&manifest_path, &config));
            validated += 1;
        }
    }

    // Workspace overrides live outside the profiles directory.
    let workspace_config = workspace.path(WorkspacePath::ConfigFile);
    if workspace_config.exists() {
        let config = ToolConfigFile::load(&workspace_config)?;
        issues.extend(validate_tool_config(&workspace_config, &config));
        validated += 1;
    }

    if issues.is_empty() {
        if validated == 0 {
            ui::info("No manifest files found to validate.");
        } else {
            ui::success(
                "Check",
                format!("Validated {validated} manifest(s) without issues."),
            );
        }
        Ok(())
    } else {
        for issue in &issues {
            let location = if let Some(tool) = &issue.tool {
                format!("{} ({tool})", issue.source.display())
            } else {
                issue.source.display().to_string()
            };
            ui::error(format!("{location}: {}", issue.message));
        }
        anyhow::bail!("Manifest validation failed ({} issue(s)).", issues.len());
    }
}
