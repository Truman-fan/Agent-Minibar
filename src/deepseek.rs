//! DeepSeek provider support through the official account balance endpoint.
//!
//! DeepSeek reports currency balances but no percentage allowance or reset
//! window. The provider therefore fills the popup credits card and deliberately
//! leaves quota windows empty.

use std::{env, time::Duration};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    instances::ProviderId,
    limits::{Credits, RateLimits},
    secrets,
    settings::ProviderKind,
    usage::UsageStatistics,
    worker::{Activator, LimitProvider, UsageProvider},
};

const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";
const SECRET_NAME: &str = "deepseek-api-key";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

pub fn is_installed(provider: ProviderId) -> bool {
    resolve_api_key(provider)
        .ok()
        .flatten()
        .is_some_and(|key| !key.trim().is_empty())
}

fn secret_name(provider: ProviderId) -> Result<String> {
    anyhow::ensure!(
        provider.kind() == ProviderKind::DeepSeek,
        "provider is not DeepSeek"
    );
    Ok(if provider.is_primary() {
        SECRET_NAME.to_owned()
    } else {
        format!("{SECRET_NAME}.{}", provider.id())
    })
}

pub fn manual_key(provider: ProviderId) -> Result<Option<String>> {
    secrets::load(&secret_name(provider)?)
}

pub fn save_manual_key(provider: ProviderId, value: Option<&str>) -> Result<()> {
    secrets::save(&secret_name(provider)?, value)
}

pub(crate) fn apply_manual_key(
    provider: ProviderId,
    value: Option<&str>,
) -> Result<secrets::EncodedRollback> {
    secrets::apply_with_rollback(&[(secret_name(provider)?, value.map(str::to_owned))])
}

pub(crate) fn restore_manual_key(rollback: secrets::EncodedRollback) -> Result<()> {
    rollback.restore()
}

pub fn key_is_configured(provider: ProviderId) -> bool {
    manual_key(provider)
        .ok()
        .flatten()
        .is_some_and(|value| !value.trim().is_empty())
}

fn env_key() -> Option<String> {
    env::var_os("DEEPSEEK_API_KEY")
        .map(|value| value.to_string_lossy().trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Additional instances are isolated and never inherit this PC's environment.
fn resolve_api_key(provider: ProviderId) -> Result<Option<String>> {
    let manual = manual_key(provider)?;
    if !provider.is_primary() {
        return Ok(manual.filter(|key| !key.trim().is_empty()));
    }
    Ok(manual.filter(|key| !key.trim().is_empty()).or_else(env_key))
}

pub struct DeepSeekClient {
    provider: ProviderId,
    agent: ureq::Agent,
}

impl DeepSeekClient {
    pub fn new(provider: ProviderId) -> Result<Self> {
        anyhow::ensure!(
            provider.kind() == ProviderKind::DeepSeek,
            "provider is not DeepSeek"
        );
        Ok(Self {
            provider,
            agent: ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT).build(),
        })
    }

    fn fetch_balance_with_key(&self, key: &str) -> Result<BalanceResponse> {
        let response = match self
            .agent
            .get(BALANCE_URL)
            .set("Authorization", &format!("Bearer {key}"))
            .set("Accept", "application/json")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(401 | 403, _)) => {
                bail!("DeepSeek rejected the API key; replace it in Settings")
            }
            Err(ureq::Error::Status(429, _)) => {
                return Err(crate::worker::rate_limit_error(
                    "DeepSeek balance request was rate limited",
                ));
            }
            Err(ureq::Error::Status(status, _)) => {
                bail!("DeepSeek balance request failed with HTTP {status}")
            }
            Err(error) => return Err(error).context("request DeepSeek account balance"),
        };
        let body = response
            .into_string()
            .context("read DeepSeek account balance response")?;
        serde_json::from_str(&body).context("parse DeepSeek account balance response")
    }

    fn read_limits_inner(&self) -> Result<RateLimits> {
        let key = resolve_api_key(self.provider)?
            .context("DeepSeek API key not found; save one in Settings or set DEEPSEEK_API_KEY")?;
        limits_from_balance(self.fetch_balance_with_key(&key)?, Utc::now())
    }
}

pub fn verify_api_key(provider: ProviderId, key: &str) -> Result<()> {
    let client = DeepSeekClient::new(provider)?;
    let balance = client.fetch_balance_with_key(key.trim())?;
    validate_balance(&balance)?;
    Ok(())
}

impl LimitProvider for DeepSeekClient {
    fn read_limits(&mut self) -> Result<RateLimits> {
        self.read_limits_inner()
    }
}

impl UsageProvider for DeepSeekClient {
    fn load_cached_usage_when_disabled(&self) -> bool {
        true
    }

    fn load_cached_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        crate::store::with_store(|store| store.load_usage_daily(self.provider, history_days))
            .or_else(|_| Ok(UsageStatistics::default()))
    }

    fn refresh_usage_statistics(&mut self, history_days: u16) -> Result<UsageStatistics> {
        crate::domestic_agents::refresh_provider_usage(self.provider, history_days)
    }

    fn refresh_without_limits(&self) -> bool {
        true
    }
}

impl Activator for DeepSeekClient {
    fn activate(&mut self) -> Result<()> {
        bail!("DeepSeek balance has no session window to activate")
    }
}

fn validate_amount(value: &str, field: &str) -> Result<()> {
    let parsed = value
        .parse::<f64>()
        .with_context(|| format!("DeepSeek {field} is not numeric"))?;
    anyhow::ensure!(
        parsed.is_finite() && parsed >= 0.0,
        "DeepSeek {field} is invalid"
    );
    Ok(())
}

fn validate_balance(response: &BalanceResponse) -> Result<()> {
    for info in &response.balance_infos {
        validate_amount(&info.total_balance, "total balance")?;
        validate_amount(&info.granted_balance, "granted balance")?;
        validate_amount(&info.topped_up_balance, "topped-up balance")?;
        anyhow::ensure!(
            !info.currency.trim().is_empty(),
            "DeepSeek balance currency is empty"
        );
    }
    Ok(())
}

fn currency_balance(info: &BalanceInfo) -> String {
    match info.currency.trim().to_ascii_uppercase().as_str() {
        "CNY" => format!("¥{}", info.total_balance),
        "USD" => format!("${}", info.total_balance),
        _ => format!("{} {}", info.total_balance, info.currency.trim()),
    }
}

fn limits_from_balance(
    response: BalanceResponse,
    sampled_at: chrono::DateTime<Utc>,
) -> Result<RateLimits> {
    validate_balance(&response)?;
    let balance = response
        .balance_infos
        .iter()
        .map(currency_balance)
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(RateLimits {
        sampled_at,
        plan_type: (!response.is_available).then(|| "API unavailable".into()),
        credits: Credits {
            has_credits: !response.balance_infos.is_empty(),
            unlimited: false,
            balance: (!balance.is_empty()).then_some(balance),
        },
        ..RateLimits::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parses_official_multi_currency_balance() {
        let response: BalanceResponse = serde_json::from_str(
            r#"{
                "is_available": true,
                "balance_infos": [
                    {"currency":"CNY","total_balance":"110.00","granted_balance":"10.00","topped_up_balance":"100.00"},
                    {"currency":"USD","total_balance":"5.25","granted_balance":"0.25","topped_up_balance":"5.00"}
                ]
            }"#,
        )
        .unwrap();
        let sampled_at = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
        let limits = limits_from_balance(response, sampled_at).unwrap();
        assert_eq!(limits.credits.balance.as_deref(), Some("¥110.00 · $5.25"));
        assert!(limits.credits.has_credits);
        assert_eq!(limits.sampled_at, sampled_at);
        assert!(limits.primary.is_empty());
        assert!(limits.secondary.is_empty());
    }

    #[test]
    fn rejects_invalid_amounts() {
        let response: BalanceResponse = serde_json::from_str(
            r#"{"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"NaN","granted_balance":"0","topped_up_balance":"0"}]}"#,
        )
        .unwrap();
        assert!(limits_from_balance(response, Utc::now()).is_err());
    }
}
