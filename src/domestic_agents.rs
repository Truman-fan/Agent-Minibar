//! Read-only local usage collection for domestic model providers.
//!
//! The collector intentionally never loads API keys and never invokes an
//! agent. It understands Kimi Code's main-agent wire log, DeepSeek Harness'
//! root and delegated-agent session logs and OpenCode's local message database. Providers
//! without a documented quota API therefore expose real token history only.

use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration as StdDuration, Instant},
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveDate, Utc};
use directories::BaseDirs;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::{
    instances::ProviderId,
    limits::RateLimits,
    provider_cli,
    settings::ProviderKind,
    store,
    usage::{DailyTokenUsage, TokenUsage, UsageStatistics, statistics_from_daily},
    worker::{Activator, LimitProvider, UsageProvider},
};

const HISTORY_DAYS: i64 = 365;
const HOURLY_HISTORY_HOURS: i64 = 48;
const MAX_JSON_LINE_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECORDS_PER_FILE: usize = 2_000_000;
const SCAN_CACHE_TTL: StdDuration = StdDuration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    KimiMain,
    DeepSeekHarness,
    OpenCode,
}

#[derive(Clone, Debug)]
struct UsageRecord {
    source: Source,
    timestamp: DateTime<Utc>,
    provider: String,
    model: String,
    usage: TokenUsage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFamily {
    DeepSeek,
    Kimi,
    VolcengineArk,
    ZhipuGlm,
    MiniMax,
    BaiduQianfan,
    TencentHunyuan,
}

impl ModelFamily {
    fn of(provider: ProviderKind) -> Option<Self> {
        match provider {
            ProviderKind::DeepSeek => Some(Self::DeepSeek),
            ProviderKind::KimiCode => Some(Self::Kimi),
            ProviderKind::VolcengineArk => Some(Self::VolcengineArk),
            ProviderKind::ZhipuGlm => Some(Self::ZhipuGlm),
            ProviderKind::MiniMaxCode => Some(Self::MiniMax),
            ProviderKind::BaiduQianfan => Some(Self::BaiduQianfan),
            ProviderKind::TencentHunyuan => Some(Self::TencentHunyuan),
            _ => None,
        }
    }

    fn matches(self, provider: &str, model: &str) -> bool {
        let identity = format!("{provider} {model}").to_ascii_lowercase();
        let has = |needles: &[&str]| needles.iter().any(|needle| identity.contains(needle));
        match self {
            Self::DeepSeek => has(&["deepseek"]),
            Self::Kimi => has(&["kimi", "moonshot"]),
            Self::VolcengineArk => has(&["doubao", "volcengine", "byteplus", "ark", "seed-"]),
            Self::ZhipuGlm => has(&["zhipu", "bigmodel", "z.ai", "glm-"]),
            Self::MiniMax => has(&["minimax"]),
            Self::BaiduQianfan => has(&["qianfan", "baidu", "ernie"]),
            Self::TencentHunyuan => has(&["hunyuan", "tencent", "codebuddy"]),
        }
    }
}

#[derive(Debug, Default)]
struct UsageRollup {
    daily: Vec<DailyTokenUsage>,
    hourly: Vec<(DateTime<Local>, TokenUsage)>,
    models: Vec<(String, NaiveDate, TokenUsage)>,
}

pub struct LocalAgentClient {
    provider: ProviderId,
}

pub struct LocalAgentActivator;

impl LocalAgentClient {
    pub fn new(provider: ProviderId) -> Result<Self> {
        anyhow::ensure!(
            provider.kind() == ProviderKind::DeepSeekHarness
                || ModelFamily::of(provider.kind()).is_some(),
            "provider has no domestic local usage adapter"
        );
        Ok(Self { provider })
    }
}

impl LimitProvider for LocalAgentClient {
    fn read_limits(&mut self) -> Result<RateLimits> {
        Ok(RateLimits {
            sampled_at: Utc::now(),
            plan_type: Some("Local agent history".into()),
            limit_name: Some("Local usage only".into()),
            ..RateLimits::default()
        })
    }
}

impl UsageProvider for LocalAgentClient {
    fn load_cached_usage_when_disabled(&self) -> bool {
        true
    }

    fn load_cached_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        store::with_store(|store| store.load_usage_daily(self.provider, history_days))
            .or_else(|_| Ok(UsageStatistics::default()))
    }

    fn refresh_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        refresh_provider_usage(self.provider, history_days)
    }

    fn refresh_without_limits(&self) -> bool {
        true
    }
}

impl Activator for LocalAgentActivator {
    fn activate(&mut self) -> Result<()> {
        bail!("local agent history has no quota window to activate")
    }
}

pub fn refresh_provider_usage(provider: ProviderId, history_days: u16) -> Result<UsageStatistics> {
    static PERSIST: Mutex<()> = Mutex::new(());
    let _guard = PERSIST
        .lock()
        .map_err(|_| anyhow::anyhow!("domestic usage persistence lock poisoned"))?;
    let now = Utc::now();
    let records = cached_records(now)?;
    let rollup = rollup(provider.kind(), &records, now);
    store::with_store(|store| {
        store.replace_usage_daily(provider, &rollup.daily)?;
        store.replace_usage_hourly(provider, &rollup.hourly)?;
        store.replace_usage_model_daily(provider, &rollup.models)
    })?;
    Ok(statistics_from_daily(&rollup.daily, history_days))
}

pub fn is_installed(provider: ProviderKind, explicit: Option<&Path>) -> bool {
    match provider {
        ProviderKind::DeepSeekHarness => {
            deepseek_harness_root().is_some_and(|root| root.is_dir())
                || cli_available(provider, explicit).is_some()
        }
        ProviderKind::KimiCode => {
            kimi_root().is_some_and(|root| root.is_dir())
                || cli_available(provider, explicit).is_some()
        }
        _ => cached_records(Utc::now()).is_ok_and(|records| {
            records
                .iter()
                .any(|record| record_matches(provider, record))
        }),
    }
}

pub fn local_source_path(provider: ProviderKind) -> Option<PathBuf> {
    match provider {
        ProviderKind::DeepSeekHarness => deepseek_harness_root().filter(|path| path.is_dir()),
        ProviderKind::KimiCode => kimi_root().filter(|path| path.is_dir()),
        _ => opencode_database_path()
            .filter(|path| path.is_file())
            .or_else(|| deepseek_harness_root().filter(|path| path.is_dir()))
            .or_else(|| kimi_root().filter(|path| path.is_dir())),
    }
}

pub fn cli_available(provider: ProviderKind, explicit: Option<&Path>) -> Option<PathBuf> {
    let names: &[&str] = match provider {
        ProviderKind::DeepSeekHarness => &["dsh.exe", "dsh.cmd", "dsh.ps1", "dsh"],
        ProviderKind::KimiCode => &["kimi.exe", "kimi.cmd", "kimi.ps1", "kimi"],
        _ => return None,
    };
    provider_cli::candidates_from(explicit, npm_launchers(names), names)
        .into_iter()
        .next()
}

fn npm_launchers(names: &[&str]) -> Vec<PathBuf> {
    let Some(appdata) = env::var_os("APPDATA") else {
        return Vec::new();
    };
    let npm = PathBuf::from(appdata).join("npm");
    names.iter().map(|name| npm.join(name)).collect()
}

struct ScanCache {
    at: Instant,
    records: Vec<UsageRecord>,
}

static SCAN_CACHE: OnceLock<Mutex<Option<ScanCache>>> = OnceLock::new();

fn scan_cache() -> &'static Mutex<Option<ScanCache>> {
    SCAN_CACHE.get_or_init(|| Mutex::new(None))
}

fn cached_records(now: DateTime<Utc>) -> Result<Vec<UsageRecord>> {
    {
        let cache = scan_cache()
            .lock()
            .map_err(|_| anyhow::anyhow!("domestic usage scan cache poisoned"))?;
        if let Some(cached) = cache.as_ref()
            && cached.at.elapsed() < SCAN_CACHE_TTL
        {
            return Ok(cached.records.clone());
        }
    }
    let records = scan_all(now);
    let mut cache = scan_cache()
        .lock()
        .map_err(|_| anyhow::anyhow!("domestic usage scan cache poisoned"))?;
    *cache = Some(ScanCache {
        at: Instant::now(),
        records: records.clone(),
    });
    Ok(records)
}

fn scan_all(now: DateTime<Utc>) -> Vec<UsageRecord> {
    let mut records = Vec::new();
    if let Some(root) = kimi_root()
        && let Err(error) = scan_kimi(&root, &mut records)
    {
        crate::logger::info(format!("Kimi Code local usage was skipped: {error:#}"));
    }
    if let Some(root) = deepseek_harness_root()
        && let Err(error) = scan_deepseek_harness(&root, &mut records)
    {
        crate::logger::info(format!(
            "DeepSeek Harness local usage was skipped: {error:#}"
        ));
    }
    if let Some(path) = opencode_database_path()
        && let Err(error) = scan_opencode(&path, now, &mut records)
    {
        crate::logger::info(format!("OpenCode domestic usage was skipped: {error:#}"));
    }
    records
}

fn record_matches(provider: ProviderKind, record: &UsageRecord) -> bool {
    if provider == ProviderKind::DeepSeekHarness {
        return record.source == Source::DeepSeekHarness;
    }
    ModelFamily::of(provider).is_some_and(|family| family.matches(&record.provider, &record.model))
}

fn rollup(provider: ProviderKind, records: &[UsageRecord], now: DateTime<Utc>) -> UsageRollup {
    let oldest = now.with_timezone(&Local).date_naive() - chrono::Duration::days(HISTORY_DAYS - 1);
    let newest = now.with_timezone(&Local).date_naive() + chrono::Duration::days(1);
    let hourly_since = now - chrono::Duration::hours(HOURLY_HISTORY_HOURS);
    let mut daily = BTreeMap::<NaiveDate, TokenUsage>::new();
    let mut hourly = BTreeMap::<DateTime<Local>, TokenUsage>::new();
    let mut models = BTreeMap::<(String, NaiveDate), TokenUsage>::new();
    for record in records
        .iter()
        .filter(|record| record_matches(provider, record))
    {
        let date = record.timestamp.with_timezone(&Local).date_naive();
        if date < oldest || date > newest {
            continue;
        }
        daily.entry(date).or_default().add_public(&record.usage);
        let model = record.model.trim();
        if !model.is_empty() {
            models
                .entry((model.to_owned(), date))
                .or_default()
                .add_public(&record.usage);
        }
        if record.timestamp >= hourly_since
            && record.timestamp <= now + chrono::Duration::minutes(5)
        {
            hourly
                .entry(crate::usage::truncate_local_hour(
                    record.timestamp.with_timezone(&Local),
                ))
                .or_default()
                .add_public(&record.usage);
        }
    }
    UsageRollup {
        daily: daily
            .into_iter()
            .map(|(date, usage)| DailyTokenUsage { date, usage })
            .collect(),
        hourly: hourly.into_iter().collect(),
        models: models
            .into_iter()
            .map(|((model, date), usage)| (model, date, usage))
            .collect(),
    }
}

fn kimi_root() -> Option<PathBuf> {
    env::var_os("KIMI_CODE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| BaseDirs::new().map(|dirs| dirs.home_dir().join(".kimi-code")))
}

fn deepseek_harness_root() -> Option<PathBuf> {
    env::var_os("DSH_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| BaseDirs::new().map(|dirs| dirs.home_dir().join(".dsh")))
}

fn opencode_database_path() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| BaseDirs::new().map(|dirs| dirs.home_dir().join(".local").join("share")))
        .map(|path| path.join("opencode").join("opencode.db"))
}

fn collect_files(root: &Path, depth: usize, output: &mut Vec<PathBuf>) -> Result<()> {
    if depth > 8 {
        return Ok(());
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("read {}", root.display())),
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_files(&entry.path(), depth + 1, output)?;
        } else if file_type.is_file() {
            output.push(entry.path());
        }
    }
    Ok(())
}

fn scan_kimi(root: &Path, output: &mut Vec<UsageRecord>) -> Result<()> {
    let mut files = Vec::new();
    collect_files(&root.join("sessions"), 0, &mut files)?;
    for path in files.into_iter().filter(|path| {
        path.file_name().and_then(|name| name.to_str()) == Some("wire.jsonl")
            && path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some("main")
    }) {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => continue,
        };
        for line in BufReader::new(file).lines().take(MAX_RECORDS_PER_FILE) {
            let Ok(line) = line else { continue };
            if line.len() > MAX_JSON_LINE_BYTES {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if value.get("type").and_then(Value::as_str) != Some("usage.record")
                || value.get("agentId").and_then(Value::as_str) != Some("main")
                || value.get("usageScope").and_then(Value::as_str) != Some("turn")
            {
                continue;
            }
            let Some(timestamp) = value
                .get("time")
                .and_then(Value::as_str)
                .and_then(parse_timestamp)
            else {
                continue;
            };
            let usage = value.get("usage").unwrap_or(&Value::Null);
            let input_other = u64_at(usage, "inputOther");
            let cache_read = u64_at(usage, "inputCacheRead");
            let cache_write = u64_at(usage, "inputCacheCreation");
            output.push(UsageRecord {
                source: Source::KimiMain,
                timestamp,
                provider: "moonshot".into(),
                model: value
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("Kimi")
                    .to_owned(),
                usage: TokenUsage {
                    input_tokens: input_other
                        .saturating_add(cache_read)
                        .saturating_add(cache_write),
                    cached_input_tokens: cache_read,
                    output_tokens: u64_at(usage, "output"),
                    requests: 1,
                    ..TokenUsage::default()
                },
            });
        }
    }
    Ok(())
}

fn session_generation(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    if matches!(name, "session.jsonl" | "session.jsonl.zstd") {
        return Some(0);
    }
    let tail = name.strip_prefix("session.v")?;
    let generation = tail
        .strip_suffix(".jsonl")
        .or_else(|| tail.strip_suffix(".jsonl.zstd"))?;
    generation.parse().ok()
}

fn scan_deepseek_harness(root: &Path, output: &mut Vec<UsageRecord>) -> Result<()> {
    let mut files = Vec::new();
    collect_files(&root.join("sessions"), 0, &mut files)?;
    let mut newest = HashMap::<PathBuf, (u32, PathBuf)>::new();
    for path in files {
        let Some(generation) = session_generation(&path) else {
            continue;
        };
        let parent = path.parent().unwrap_or(root).to_path_buf();
        let replace = newest
            .get(&parent)
            .is_none_or(|(known, _)| generation > *known);
        if replace {
            newest.insert(parent, (generation, path));
        }
    }
    for (_, (_, path)) in newest {
        scan_deepseek_session(&path, output)?;
    }
    Ok(())
}

fn scan_deepseek_session(path: &Path, output: &mut Vec<UsageRecord>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader: Box<dyn BufRead> = if path.extension().and_then(|ext| ext.to_str()) == Some("zstd")
    {
        Box::new(BufReader::new(
            zstd::stream::read::Decoder::new(file)
                .with_context(|| format!("decode {}", path.display()))?,
        ))
    } else {
        Box::new(BufReader::new(file))
    };
    let mut accepted_session = false;
    let mut session_time = None;
    let mut current_provider = String::new();
    let mut current_model = String::new();
    let mut session_records = Vec::<UsageRecord>::new();
    let mut replacement_slot = None::<(u64, u64, usize)>;
    for line in reader.lines().take(MAX_RECORDS_PER_FILE) {
        let Ok(line) = line else { continue };
        if line.len() > MAX_JSON_LINE_BYTES {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if event_type == "session" {
            accepted_session = !value
                .get("isSeeded")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            session_time = value
                .get("createdAt")
                .and_then(Value::as_i64)
                .and_then(timestamp_from_number);
            continue;
        }
        if !accepted_session {
            continue;
        }
        if event_type == "request/context" {
            current_provider = value
                .pointer("/data/provider")
                .and_then(Value::as_str)
                .unwrap_or(&current_provider)
                .to_owned();
            current_model = value
                .pointer("/data/model")
                .and_then(Value::as_str)
                .unwrap_or(&current_model)
                .to_owned();
        }
        if event_type == "llm/retry-started" {
            let turn = value.pointer("/data/turn").and_then(Value::as_u64);
            let step = value.pointer("/data/step").and_then(Value::as_u64);
            if replacement_slot
                .as_ref()
                .is_some_and(|(known_turn, known_step, _)| {
                    Some(*known_turn) == turn && Some(*known_step) == step
                })
            {
                replacement_slot = None;
            }
            continue;
        }
        if let Some(source) = value.pointer("/data/message/source") {
            current_provider = source
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(&current_provider)
                .to_owned();
            current_model = source
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(&current_model)
                .to_owned();
        }
        if !matches!(event_type, "assistant/message" | "assistant/attempt") {
            continue;
        }
        let Some(usage) = usage_from_harness_event(&value) else {
            continue;
        };
        let Some(timestamp) = event_timestamp(&value).or(session_time) else {
            continue;
        };
        let input = u64_at(usage, "inputTokens");
        let cache_read = u64_at(usage, "cacheReadTokens");
        let cache_write = u64_at(usage, "cacheWriteTokens");
        let record = UsageRecord {
            source: Source::DeepSeekHarness,
            timestamp,
            provider: current_provider.clone(),
            model: current_model.clone(),
            usage: TokenUsage {
                input_tokens: input.saturating_add(cache_read).saturating_add(cache_write),
                cached_input_tokens: cache_read,
                output_tokens: u64_at(usage, "outputTokens"),
                requests: 1,
                ..TokenUsage::default()
            },
        };
        let turn = value
            .pointer("/data/turn")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        let step = value
            .pointer("/data/step")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        if let Some((known_turn, known_step, index)) = replacement_slot
            && known_turn == turn
            && known_step == step
        {
            session_records[index] = record;
        } else {
            let index = session_records.len();
            session_records.push(record);
            replacement_slot = Some((turn, step, index));
        }
    }
    output.extend(session_records);
    Ok(())
}

fn usage_from_harness_event(value: &Value) -> Option<&Value> {
    if let Some(usage) = value.pointer("/data/usage") {
        return Some(usage);
    }
    value
        .pointer("/data/stream")?
        .as_array()?
        .iter()
        .rev()
        .find_map(|entry| {
            (entry.pointer("/chunk/type").and_then(Value::as_str) == Some("usage"))
                .then(|| entry.pointer("/chunk/usage"))
                .flatten()
        })
}

fn scan_opencode(path: &Path, now: DateTime<Utc>, output: &mut Vec<UsageRecord>) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open OpenCode database read-only at {}", path.display()))?;
    let oldest_ms = (now - chrono::Duration::days(HISTORY_DAYS)).timestamp_millis();
    let mut statement = connection.prepare(
        "SELECT
             CAST(COALESCE(json_extract(data, '$.time.created'), time_created) AS INTEGER),
             COALESCE(json_extract(data, '$.providerID'), ''),
             COALESCE(json_extract(data, '$.modelID'), ''),
             COALESCE(CAST(json_extract(data, '$.tokens.input') AS INTEGER), 0),
             COALESCE(CAST(json_extract(data, '$.tokens.cache.read') AS INTEGER), 0),
             COALESCE(CAST(json_extract(data, '$.tokens.output') AS INTEGER), 0),
             COALESCE(CAST(json_extract(data, '$.tokens.reasoning') AS INTEGER), 0)
         FROM message
         WHERE json_valid(data)
           AND json_extract(data, '$.role') = 'assistant'
           AND CAST(COALESCE(json_extract(data, '$.time.created'), time_created) AS INTEGER) >= ?1
         ORDER BY 1 ASC",
    )?;
    let rows = statement.query_map([oldest_ms], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            non_negative(row.get::<_, i64>(3)?),
            non_negative(row.get::<_, i64>(4)?),
            non_negative(row.get::<_, i64>(5)?),
            non_negative(row.get::<_, i64>(6)?),
        ))
    })?;
    for row in rows {
        let (timestamp_ms, provider, model, input, cache, output_tokens, reasoning) = row?;
        let Some(timestamp) = DateTime::<Utc>::from_timestamp_millis(timestamp_ms) else {
            continue;
        };
        output.push(UsageRecord {
            source: Source::OpenCode,
            timestamp,
            provider,
            model,
            usage: TokenUsage {
                input_tokens: input,
                cached_input_tokens: cache.min(input),
                output_tokens: output_tokens.saturating_add(reasoning),
                requests: 1,
                ..TokenUsage::default()
            },
        });
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn timestamp_from_number(value: i64) -> Option<DateTime<Utc>> {
    if value.abs() >= 10_000_000_000 {
        DateTime::<Utc>::from_timestamp_millis(value)
    } else {
        DateTime::<Utc>::from_timestamp(value, 0)
    }
}

fn event_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    ["timestamp", "createdAt", "time"]
        .into_iter()
        .find_map(|field| match value.get(field) {
            Some(Value::String(value)) => parse_timestamp(value),
            Some(Value::Number(value)) => value.as_i64().and_then(timestamp_from_number),
            _ => None,
        })
}

fn u64_at(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn non_negative(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn model_families_cover_current_domestic_aliases() {
        assert!(ModelFamily::DeepSeek.matches("openai", "deepseek-v3"));
        assert!(ModelFamily::Kimi.matches("moonshot", "kimi-k2"));
        assert!(ModelFamily::VolcengineArk.matches("volcengine", "doubao-seed-code"));
        assert!(ModelFamily::ZhipuGlm.matches("z.ai", "glm-4.7"));
        assert!(ModelFamily::MiniMax.matches("minimax", "MiniMax-M2"));
        assert!(ModelFamily::BaiduQianfan.matches("qianfan", "ernie-5"));
        assert!(ModelFamily::TencentHunyuan.matches("tencent", "hunyuan-code"));
    }

    #[test]
    fn kimi_scans_only_main_turn_usage() {
        let root = tempdir().unwrap();
        let main = root.path().join("sessions/ws/s/agents/main");
        let child = root.path().join("sessions/ws/s/agents/agent-child");
        fs::create_dir_all(&main).unwrap();
        fs::create_dir_all(&child).unwrap();
        let row = r#"{"type":"usage.record","agentId":"main","model":"kimi-k2","usage":{"inputOther":10,"output":3,"inputCacheRead":4,"inputCacheCreation":2},"usageScope":"turn","time":"2026-11-07T01:00:00Z"}"#;
        writeln!(File::create(main.join("wire.jsonl")).unwrap(), "{row}").unwrap();
        writeln!(File::create(child.join("wire.jsonl")).unwrap(), "{row}").unwrap();
        let mut records = Vec::new();
        scan_kimi(root.path(), &mut records).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].usage.input_tokens, 16);
        assert_eq!(records[0].usage.cached_input_tokens, 4);
    }

    #[test]
    fn harness_includes_delegated_sessions_but_excludes_seeded_sessions() {
        let root = tempdir().unwrap();
        for (name, seeded, depth) in [("root", false, 0), ("seed", true, 0), ("child", false, 1)] {
            let dir = root.path().join("sessions").join(name);
            fs::create_dir_all(&dir).unwrap();
            let mut file = File::create(dir.join("session.v2.jsonl")).unwrap();
            writeln!(file, "{}", serde_json::json!({"type":"session","createdAt":1794013200000_i64,"isSeeded":seeded,"delegationDepth":depth})).unwrap();
            writeln!(file, "{}", serde_json::json!({"type":"assistant/message","data":{"message":{"source":{"provider":"deepseek","model":"deepseek-v3"}},"usage":{"inputTokens":10,"outputTokens":2,"cacheReadTokens":3}}})).unwrap();
        }
        let mut records = Vec::new();
        scan_deepseek_harness(root.path(), &mut records).unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|record| record.usage.input_tokens == 13));
    }

    #[test]
    fn harness_replaces_same_attempt_samples_but_accumulates_retries() {
        let root = tempdir().unwrap();
        let dir = root.path().join("sessions/root");
        fs::create_dir_all(&dir).unwrap();
        let mut file = File::create(dir.join("session.v4.jsonl")).unwrap();
        writeln!(file, "{}", serde_json::json!({"type":"session","createdAt":1794013200000_i64,"isSeeded":false,"delegationDepth":0})).unwrap();
        writeln!(file, "{}", serde_json::json!({"type":"request/context","data":{"provider":"deepseek","model":"deepseek-v3"}})).unwrap();
        let attempt = |input| serde_json::json!({"type":"assistant/attempt","data":{"turn":1,"step":1,"stream":[{"type":"chunk","chunk":{"type":"usage","usage":{"inputTokens":input,"outputTokens":2}}}]}});
        writeln!(file, "{}", attempt(10)).unwrap();
        writeln!(file, "{}", attempt(12)).unwrap();
        writeln!(
            file,
            "{}",
            serde_json::json!({"type":"llm/retry-started","data":{"turn":1,"step":1}})
        )
        .unwrap();
        writeln!(file, "{}", attempt(20)).unwrap();

        let mut records = Vec::new();
        scan_deepseek_harness(root.path(), &mut records).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].usage.input_tokens, 12);
        assert_eq!(records[1].usage.input_tokens, 20);
    }

    #[test]
    fn opencode_reads_domestic_tokens_without_requiring_cost() {
        let root = tempdir().unwrap();
        let path = root.path().join("opencode.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE message (data TEXT NOT NULL, time_created INTEGER NOT NULL);",
            )
            .unwrap();
        let timestamp = 1_794_013_200_000_i64;
        let value = serde_json::json!({
            "role":"assistant",
            "providerID":"volcengine",
            "modelID":"doubao-seed-code",
            "time":{"created":timestamp},
            "tokens":{"input":100,"cache":{"read":40},"output":20,"reasoning":5}
        });
        connection
            .execute(
                "INSERT INTO message(data, time_created) VALUES (?1, ?2)",
                rusqlite::params![value.to_string(), timestamp],
            )
            .unwrap();
        drop(connection);

        let now = Utc.with_ymd_and_hms(2026, 11, 7, 3, 0, 0).unwrap();
        let mut records = Vec::new();
        scan_opencode(&path, now, &mut records).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].usage.total_tokens(), 125);
        assert_eq!(records[0].usage.cached_input_tokens, 40);
        assert!(record_matches(ProviderKind::VolcengineArk, &records[0]));
    }

    #[test]
    fn rollup_keeps_harness_and_model_views_separate() {
        let now = Utc.with_ymd_and_hms(2026, 11, 7, 3, 0, 0).unwrap();
        let records = vec![UsageRecord {
            source: Source::DeepSeekHarness,
            timestamp: now,
            provider: "moonshot".into(),
            model: "kimi-k2".into(),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 2,
                requests: 1,
                ..Default::default()
            },
        }];
        assert_eq!(
            rollup(ProviderKind::DeepSeekHarness, &records, now)
                .daily
                .len(),
            1
        );
        assert_eq!(rollup(ProviderKind::KimiCode, &records, now).daily.len(), 1);
        assert!(
            rollup(ProviderKind::DeepSeek, &records, now)
                .daily
                .is_empty()
        );
    }
}
