//! Qwen Code discovery and local token-usage statistics.
//!
//! Qwen Code writes one schema-versioned JSONL record per API response under
//! its runtime `usage` directory. Agent Minibar reads those records without
//! invoking the CLI or sending a request with the user's credentials. Qwen's
//! public configuration currently exposes the account mode but not a stable
//! remaining-quota endpoint, so this provider deliberately returns no quota
//! percentage or reset window.

use std::{
    collections::{BTreeMap, HashSet},
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveDate, Utc};
use directories::BaseDirs;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    instances::ProviderId,
    limits::RateLimits,
    provider_cli, store,
    usage::{DailyTokenUsage, TokenUsage, UsageStatistics, statistics_from_daily},
    worker::{Activator, LimitProvider, UsageProvider},
};

const CLI_NAMES: &[&str] = &[
    "qwen.cmd",
    "qwen.ps1",
    "qwen.exe",
    "qwen-code.cmd",
    "qwen-code.ps1",
    "qwen-code.exe",
];
const LOCAL_HISTORY_DAYS: i64 = 365;
const HOURLY_HISTORY_HOURS: i64 = 48;

pub struct QwenCodeClient {
    provider: ProviderId,
}

pub struct QwenCodeActivator;

impl QwenCodeClient {
    pub fn new(provider: ProviderId) -> Self {
        Self { provider }
    }
}

impl LimitProvider for QwenCodeClient {
    fn read_limits(&mut self) -> Result<RateLimits> {
        Ok(RateLimits {
            sampled_at: Utc::now(),
            plan_type: detected_account_mode(),
            limit_name: Some("Local usage only".into()),
            ..RateLimits::default()
        })
    }
}

impl UsageProvider for QwenCodeClient {
    fn load_cached_usage_when_disabled(&self) -> bool {
        true
    }

    fn load_cached_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        store::with_store(|store| store.load_usage_daily(self.provider, history_days))
    }

    fn refresh_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        // A retiring worker can finish after its replacement starts. Serialize
        // complete rescans so their replace transactions cannot interleave.
        static SCAN: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _scan = SCAN
            .lock()
            .map_err(|_| anyhow::anyhow!("Qwen Code scan lock poisoned"))?;
        let root = usage_root().context("Qwen Code home directory is unavailable")?;
        let rollup = scan_usage(&root, Utc::now())?;
        store::with_store(|store| {
            store.replace_usage_daily(self.provider, &rollup.daily)?;
            store.replace_usage_hourly(self.provider, &rollup.hourly)?;
            store.replace_usage_model_daily(self.provider, &rollup.models)
        })?;
        Ok(statistics_from_daily(&rollup.daily, history_days))
    }

    fn refresh_without_limits(&self) -> bool {
        true
    }
}

impl Activator for QwenCodeActivator {
    fn activate(&mut self) -> Result<()> {
        bail!("Qwen Code local usage has no session window to activate")
    }
}

/// Qwen Code is useful when either its CLI or one of its read-only local data
/// sources exists. A `.env` file counts as configuration without reading any
/// key values from it.
pub fn is_installed(explicit: Option<&Path>) -> bool {
    cli_available(explicit).is_some()
        || settings_path().is_some_and(|path| path.is_file())
        || usage_root().is_some_and(|path| path.is_dir())
        || env_paths().into_iter().any(|path| path.is_file())
}

pub fn cli_available(explicit: Option<&Path>) -> Option<PathBuf> {
    provider_cli::candidates_from(explicit, npm_launchers(), CLI_NAMES)
        .into_iter()
        .next()
}

/// A display-only source path for Settings. This never reads secret values.
pub fn local_source_path() -> Option<PathBuf> {
    settings_path()
        .filter(|path| path.is_file())
        .or_else(|| usage_root().filter(|path| path.is_dir()))
        .or_else(|| env_paths().into_iter().find(|path| path.is_file()))
}

pub fn settings_path() -> Option<PathBuf> {
    qwen_home().map(|home| home.join("settings.json"))
}

pub fn usage_root() -> Option<PathBuf> {
    runtime_root().map(|root| root.join("usage"))
}

/// Names the active Qwen account/backend from non-secret settings fields.
/// Missing or malformed settings simply leave the label unavailable.
pub fn detected_account_mode() -> Option<String> {
    settings_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .as_ref()
        .and_then(account_mode_from_settings)
        .or_else(account_mode_from_environment)
}

fn qwen_home() -> Option<PathBuf> {
    env::var_os("QWEN_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .and_then(resolve_config_path)
        .or_else(|| BaseDirs::new().map(|dirs| dirs.home_dir().join(".qwen")))
}

fn runtime_root() -> Option<PathBuf> {
    env::var_os("QWEN_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .and_then(resolve_config_path)
        .or_else(qwen_home)
}

fn resolve_config_path(path: PathBuf) -> Option<PathBuf> {
    let path = if path == Path::new("~") {
        BaseDirs::new()?.home_dir().to_path_buf()
    } else if let Ok(rest) = path.strip_prefix("~") {
        BaseDirs::new()?.home_dir().join(rest)
    } else {
        path
    };
    if path.is_absolute() {
        Some(path)
    } else {
        env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

fn env_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = qwen_home() {
        paths.push(home.join(".env"));
    }
    if let Ok(cwd) = env::current_dir() {
        paths.push(cwd.join(".qwen").join(".env"));
    }
    paths
}

fn npm_launchers() -> Vec<PathBuf> {
    let Some(appdata) = env::var_os("APPDATA") else {
        return Vec::new();
    };
    let npm = PathBuf::from(appdata).join("npm");
    CLI_NAMES.iter().map(|name| npm.join(name)).collect()
}

fn account_mode_from_settings(settings: &Value) -> Option<String> {
    let auth_type = settings
        .pointer("/security/auth/selectedType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if auth_type == Some("qwen-oauth") {
        return Some("Qwen Plus / Pro subscription".into());
    }

    let model = settings
        .pointer("/model/name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let selected = auth_type.and_then(|auth| {
        settings
            .get("modelProviders")?
            .get(auth)?
            .as_array()?
            .iter()
            .find(|entry| entry.get("id").and_then(Value::as_str) == model)
    });
    let base_url = selected
        .and_then(|entry| entry.get("baseUrl"))
        .and_then(Value::as_str)
        .or_else(|| {
            settings
                .pointer("/security/auth/baseUrl")
                .and_then(Value::as_str)
        });
    let env_key = selected
        .and_then(|entry| entry.get("envKey"))
        .and_then(Value::as_str);
    classify_account_mode(auth_type, base_url, env_key)
}

fn account_mode_from_environment() -> Option<String> {
    // Endpoint URLs are not credentials. Secret variables are checked only
    // for presence; their values are never loaded by this provider.
    let base_url = env::var("OPENAI_BASE_URL").ok()?;
    let env_key = [
        "BAILIAN_CODING_PLAN_API_KEY",
        "BAILIAN_TOKEN_PLAN_API_KEY",
        "QWENCLOUD_TOKEN_PLAN_API_KEY",
        "QWENCLOUD_API_KEY",
        "DASHSCOPE_API_KEY",
        "OPENAI_API_KEY",
    ]
    .into_iter()
    .find(|name| env::var_os(name).is_some());
    classify_account_mode(Some("openai"), Some(&base_url), env_key)
}

fn classify_account_mode(
    auth_type: Option<&str>,
    base_url: Option<&str>,
    env_key: Option<&str>,
) -> Option<String> {
    let url = base_url.unwrap_or_default().trim().to_ascii_lowercase();
    let key = env_key.unwrap_or_default().trim().to_ascii_uppercase();
    let label = if url.contains("coding.dashscope.aliyuncs.com")
        || url.contains("coding-intl.dashscope.aliyuncs.com")
        || key == "BAILIAN_CODING_PLAN_API_KEY"
    {
        "Alibaba ModelStudio Coding Plan"
    } else if (url.contains("token-plan.") && url.contains("maas.aliyuncs.com"))
        || key == "BAILIAN_TOKEN_PLAN_API_KEY"
    {
        "Alibaba ModelStudio Token Plan"
    } else if (url.contains("token-plan.maas.qianwenaiapi.com")
        || url.contains("token-plan.maas.qwencloudapi.com"))
        || key == "QWENCLOUD_TOKEN_PLAN_API_KEY"
    {
        "QwenCloud Token Plan"
    } else if url.contains("maas.qianwenaiapi.com")
        || url.contains("maas.qwencloudapi.com")
        || key == "QWENCLOUD_API_KEY"
    {
        "QwenCloud standard API"
    } else if url.contains("dashscope.aliyuncs.com")
        || url.contains("dashscope-intl.aliyuncs.com")
        || key == "DASHSCOPE_API_KEY"
    {
        "Alibaba ModelStudio standard API"
    } else if !url.is_empty() {
        "Third-party or custom provider"
    } else {
        match auth_type? {
            "qwen-oauth" => "Qwen Plus / Pro subscription",
            "openai" | "openai-responses" | "anthropic" | "gemini" | "vertex-ai" => {
                "Third-party or custom provider"
            }
            _ => "Qwen Code local history",
        }
    };
    Some(label.into())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageRecord {
    schema_version: u32,
    id: String,
    timestamp: String,
    local_date: String,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    thoughts_tokens: u64,
    total_tokens: u64,
}

impl UsageRecord {
    fn parsed(&self) -> Option<(NaiveDate, DateTime<Utc>, TokenUsage)> {
        if self.schema_version != 1 || self.id.trim().is_empty() {
            return None;
        }
        let date = NaiveDate::parse_from_str(&self.local_date, "%Y-%m-%d").ok()?;
        let timestamp = DateTime::parse_from_rfc3339(&self.timestamp)
            .ok()?
            .with_timezone(&Utc);
        let input = self.input_tokens;
        let reported_output = self.output_tokens.saturating_add(self.thoughts_tokens);
        // Match Qwen Code's authoritative total when present while retaining
        // output/thought counts if an older backend omitted that total.
        let output = if self.total_tokens > 0 {
            self.total_tokens.saturating_sub(input).max(reported_output)
        } else {
            reported_output
        };
        Some((
            date,
            timestamp,
            TokenUsage {
                input_tokens: input,
                cached_input_tokens: self.cached_tokens.min(input),
                output_tokens: output,
                requests: 1,
                ..TokenUsage::default()
            },
        ))
    }
}

#[derive(Debug, Default)]
struct UsageRollup {
    daily: Vec<DailyTokenUsage>,
    hourly: Vec<(DateTime<Local>, TokenUsage)>,
    models: Vec<(String, NaiveDate, TokenUsage)>,
}

fn scan_usage(root: &Path, now: DateTime<Utc>) -> Result<UsageRollup> {
    let mut files = usage_files(root)?;
    files.sort();
    let oldest_date =
        now.with_timezone(&Local).date_naive() - chrono::Duration::days(LOCAL_HISTORY_DAYS - 1);
    let newest_date = now.with_timezone(&Local).date_naive() + chrono::Duration::days(1);
    let hourly_since = now - chrono::Duration::hours(HOURLY_HISTORY_HOURS);
    let mut seen = HashSet::new();
    let mut daily = BTreeMap::<NaiveDate, TokenUsage>::new();
    let mut hourly = BTreeMap::<DateTime<Local>, TokenUsage>::new();
    let mut models = BTreeMap::<(String, NaiveDate), TokenUsage>::new();

    for path in files {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                crate::logger::info(format!(
                    "Qwen Code usage file {} was skipped: {error}",
                    path.display()
                ));
                continue;
            }
        };
        for line in BufReader::new(file).lines() {
            let Ok(line) = line else { continue };
            let Ok(record) = serde_json::from_str::<UsageRecord>(&line) else {
                continue;
            };
            let Some((date, timestamp, usage)) = record.parsed() else {
                continue;
            };
            if !seen.insert(record.id.clone()) || date < oldest_date || date > newest_date {
                continue;
            }
            daily.entry(date).or_default().add_public(&usage);
            let model = record.model.trim();
            if !model.is_empty() {
                models
                    .entry((model.to_owned(), date))
                    .or_default()
                    .add_public(&usage);
            }
            if timestamp >= hourly_since && timestamp <= now + chrono::Duration::minutes(5) {
                let hour = crate::usage::truncate_local_hour(timestamp.with_timezone(&Local));
                hourly.entry(hour).or_default().add_public(&usage);
            }
        }
    }

    Ok(UsageRollup {
        daily: daily
            .into_iter()
            .map(|(date, usage)| DailyTokenUsage { date, usage })
            .collect(),
        hourly: hourly.into_iter().collect(),
        models: models
            .into_iter()
            .map(|((model, date), usage)| (model, date, usage))
            .collect(),
    })
}

fn usage_files(root: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", root.display())),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("token-usage-") && name.ends_with(".jsonl") {
            files.push(entry.path());
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn classifies_current_official_account_modes_without_credentials() {
        assert_eq!(
            classify_account_mode(Some("qwen-oauth"), None, None).as_deref(),
            Some("Qwen Plus / Pro subscription")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://coding.dashscope.aliyuncs.com/v1"),
                Some("BAILIAN_CODING_PLAN_API_KEY")
            )
            .as_deref(),
            Some("Alibaba ModelStudio Coding Plan")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://token-plan.maas.qwencloudapi.com/v1"),
                Some("QWENCLOUD_TOKEN_PLAN_API_KEY")
            )
            .as_deref(),
            Some("QwenCloud Token Plan")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://token-plan.cn-beijing.maas.aliyuncs.com/v1"),
                Some("BAILIAN_TOKEN_PLAN_API_KEY")
            )
            .as_deref(),
            Some("Alibaba ModelStudio Token Plan")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://maas.qwencloudapi.com/compatible-mode/v1"),
                Some("QWENCLOUD_API_KEY")
            )
            .as_deref(),
            Some("QwenCloud standard API")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
                Some("DASHSCOPE_API_KEY")
            )
            .as_deref(),
            Some("Alibaba ModelStudio standard API")
        );
        assert_eq!(
            classify_account_mode(
                Some("openai"),
                Some("https://example.invalid/v1"),
                Some("DO_NOT_DISPLAY_THIS_KEY")
            )
            .as_deref(),
            Some("Third-party or custom provider")
        );
    }

    #[test]
    fn reads_active_model_provider_without_returning_env_values() {
        let settings: Value = serde_json::from_str(
            r#"{
              "security":{"auth":{"selectedType":"openai"}},
              "model":{"name":"qwen3-coder-plus"},
              "modelProviders":{"openai":[{
                "id":"qwen3-coder-plus",
                "baseUrl":"https://coding-intl.dashscope.aliyuncs.com/v1",
                "envKey":"BAILIAN_CODING_PLAN_API_KEY"
              }]},
              "env":{"BAILIAN_CODING_PLAN_API_KEY":"secret-must-not-escape"}
            }"#,
        )
        .unwrap();
        let mode = account_mode_from_settings(&settings).unwrap();
        assert_eq!(mode, "Alibaba ModelStudio Coding Plan");
        assert!(!mode.contains("secret"));
    }

    #[test]
    fn scans_schema_one_records_and_deduplicates_ids() {
        let root = tempdir().unwrap();
        let path = root.path().join("token-usage-2026-11.jsonl");
        let mut file = File::create(path).unwrap();
        let first = r#"{"schemaVersion":1,"id":"response-a","timestamp":"2026-11-07T01:15:00Z","localDate":"2026-11-07","localMonth":"2026-11","sessionId":"s1","model":"Qwen3-Coder","authType":"openai","source":"main","inputTokens":100,"outputTokens":20,"cachedTokens":40,"thoughtsTokens":5,"totalTokens":125,"apiDurationMs":1000}"#;
        writeln!(file, "{first}").unwrap();
        writeln!(file, "{first}").unwrap();
        writeln!(file, "not json").unwrap();
        file.write_all(
            b"{\"schemaVersion\":2,\"id\":\"future-schema\",\"timestamp\":\"2026-11-07T02:00:00Z\",\"localDate\":\"2026-11-07\",\"model\":\"ignored\",\"inputTokens\":999,\"outputTokens\":0,\"cachedTokens\":0,\"thoughtsTokens\":0,\"totalTokens\":999}\n",
        )
        .unwrap();

        let now = Utc.with_ymd_and_hms(2026, 11, 7, 3, 0, 0).unwrap();
        let rollup = scan_usage(root.path(), now).unwrap();
        assert_eq!(rollup.daily.len(), 1);
        assert_eq!(rollup.daily[0].usage.input_tokens, 100);
        assert_eq!(rollup.daily[0].usage.cached_input_tokens, 40);
        assert_eq!(rollup.daily[0].usage.output_tokens, 25);
        assert_eq!(rollup.daily[0].usage.requests, 1);
        assert_eq!(rollup.models[0].0, "Qwen3-Coder");
        assert_eq!(rollup.hourly.len(), 1);
    }
}
