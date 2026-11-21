use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;

use crate::{
    limits::RateLimits,
    usage::UsageStatistics,
    worker::{Activator, LimitProvider, UsageProvider},
};

pub(crate) const USAGE_GUIDE: &str = "https://www.workbuddy.cn/docs/workbuddy/Usage";

pub(crate) struct WorkBuddy;

impl LimitProvider for WorkBuddy {
    fn read_limits(&mut self) -> Result<RateLimits> {
        Ok(RateLimits {
            sampled_at: Utc::now(),
            ..RateLimits::default()
        })
    }
}

impl UsageProvider for WorkBuddy {
    fn load_cached_usage_statistics(&mut self, _: u16) -> Result<UsageStatistics> {
        Ok(UsageStatistics::default())
    }

    fn refresh_usage_statistics(&mut self, _: u16) -> Result<UsageStatistics> {
        Ok(UsageStatistics::default())
    }
}

impl Activator for WorkBuddy {
    fn activate(&mut self) -> Result<()> {
        anyhow::bail!("WorkBuddy does not expose a quota activation window")
    }
}

pub(crate) fn installed_app(path: Option<&Path>) -> Option<PathBuf> {
    fn executable(path: &Path) -> Option<PathBuf> {
        let path = if path.is_dir() {
            path.join("WorkBuddy.exe")
        } else {
            path.to_owned()
        };
        (path.is_file()
            && path
                .file_name()?
                .to_str()?
                .eq_ignore_ascii_case("WorkBuddy.exe"))
        .then_some(path)
    }
    path.and_then(executable).or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .into_iter()
            .map(|root| PathBuf::from(root).join("Programs"))
            .chain(std::env::var_os("ProgramFiles").map(PathBuf::from))
            .map(|root| root.join("WorkBuddy"))
            .find_map(|path| executable(&path))
    })
}
