//! Product identity, repository locations and legacy-data compatibility.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;

pub const PRODUCT_NAME: &str = "Agent Minibar";
pub const MAINTAINER_NAME: &str = "Truman_show";
pub const MAINTAINER_URL: &str = "https://github.com/Truman-fan";
pub const GITHUB_REPO_URL: &str = "https://github.com/Truman-fan/Agent-Minibar";
pub const GITEE_REPO_URL: &str = "https://gitee.com/Truman-fan/Agent-Minibar";
pub const UPSTREAM_REPO_URL: &str = "https://github.com/vertopolkaLF/codex-minibar";
pub const UPSTREAM_AUTHOR: &str = "Pavel Leontyev / @vertopolkaLF";

const QUALIFIER: &str = "io.github";
const ORGANIZATION: &str = "Truman_show";
const APPLICATION: &str = "Agent Minibar";

const LEGACY_QUALIFIER: &str = "dev";
const LEGACY_ORGANIZATION: &str = "Codex Minibar";
const LEGACY_APPLICATION: &str = "Codex Minibar";

pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .context("could not resolve the Agent Minibar application directory")
}

fn legacy_project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(LEGACY_QUALIFIER, LEGACY_ORGANIZATION, LEGACY_APPLICATION)
        .context("could not resolve the legacy Codex Minibar application directory")
}

/// Copies data from the upstream application's identity on first launch.
///
/// Existing Agent Minibar files always win and the legacy tree is retained, so
/// users can roll back to Codex Minibar without losing settings or credentials.
pub fn migrate_legacy_storage() -> Result<usize> {
    if crate::fake::enabled() {
        return Ok(0);
    }
    let current = project_dirs()?;
    let legacy = legacy_project_dirs()?;
    let pairs = [
        (legacy.config_dir(), current.config_dir()),
        (legacy.data_dir(), current.data_dir()),
        (legacy.data_local_dir(), current.data_local_dir()),
        (legacy.cache_dir(), current.cache_dir()),
    ];
    let mut seen = HashSet::<(PathBuf, PathBuf)>::new();
    let mut copied = 0;
    for (source, destination) in pairs {
        let pair = (source.to_path_buf(), destination.to_path_buf());
        if pair.0 == pair.1 || !seen.insert(pair.clone()) || !pair.0.exists() {
            continue;
        }
        copied += copy_missing_tree(&pair.0, &pair.1)?;
    }
    Ok(copied)
}

fn copy_missing_tree(source: &Path, destination: &Path) -> Result<usize> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("inspect legacy path {}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        if destination.exists() {
            return Ok(0);
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create migration directory {}", parent.display()))?;
        }
        fs::copy(source, destination).with_context(|| {
            format!(
                "migrate legacy file {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        return Ok(1);
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "create Agent Minibar migration directory {}",
            destination.display()
        )
    })?;
    let mut copied = 0;
    for entry in
        fs::read_dir(source).with_context(|| format!("read legacy path {}", source.display()))?
    {
        let entry = entry.with_context(|| format!("read entry under {}", source.display()))?;
        copied += copy_missing_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_copies_only_missing_regular_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("old");
        let destination = temp.path().join("new");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("settings.toml"), "old").unwrap();
        fs::write(source.join("nested/store.sqlite"), "db").unwrap();
        fs::write(destination.join("settings.toml"), "new").unwrap();

        assert_eq!(copy_missing_tree(&source, &destination).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(destination.join("settings.toml")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(destination.join("nested/store.sqlite")).unwrap(),
            "db"
        );
        assert_eq!(copy_missing_tree(&source, &destination).unwrap(), 0);
    }
}
