# Domestic provider expansion plan

This plan adds Chinese AI services without inventing quota percentages or
using subscription credentials outside their documented purpose. A provider in
Agent Minibar represents the account or plan that owns a quota; the model used
for a request is secondary metadata. This distinction matters because a single
Coding Plan can share quota across Qwen, Doubao, DeepSeek, Kimi, GLM and other
models.

## Product rules

1. Prefer an official, documented quota or balance API.
2. Otherwise read supported local agent configuration and session history.
3. If only local consumption is known, label it as usage rather than remaining
   quota. Do not infer a percentage from an undocumented plan limit.
4. Never send a probe request merely to discover quota, and never use a Coding
   Plan key through an endpoint or client category that its terms prohibit.
5. Keep secrets in the existing protected secret store. A key discovered in an
   agent configuration is used in memory only, is never copied to settings or
   logs, and is always masked in the UI.
6. Each compact floating circle represents one provider account and shows at
   most two real remaining-quota rings. Agents and sessions are never circle
   targets. The detail card shows every available window and balance.

## Provider matrix

| Provider surface | Supported account modes | Authoritative data available now | Compact display | Initial status |
| --- | --- | --- | --- | --- |
| DeepSeek API | API key; key discovered from supported agent configuration | Official `/user/balance`: availability plus CNY/USD total, granted and topped-up balances | Balance badge/card; no percentage ring unless a spending cap is explicitly configured | Phase 1 |
| Qwen Code | Qwen Plus/Pro OAuth subscription, QwenCloud Token Plan/standard API, Alibaba ModelStudio Coding Plan/Token Plan/standard API, third-party or custom provider | Official local token ledger and non-secret auth/plan metadata. Current public Qwen Code material does not expose a stable remaining-subscription-quota endpoint | “No quota API” status card; quota ring only after an official source is verified | Phase 2 implemented |
| Volcengine Ark | Normal API key, Agent Plan, Coding Plan | Coding Plan documents 5-hour, weekly and monthly windows, but the key is restricted to supported interactive coding tools and the public plan page directs users to the console for usage | Three windows in detail card only when obtained from a documented source; two selected windows in the circle | Phase 3 |
| Doubao model | Model selected through Volcengine Ark | Uses the owning Ark/API/Plan provider's data; it is not an independent quota account | Doubao model badge/icon on the Ark provider; no duplicate provider or double counting | Phase 3 |

## Implemented local-agent adapters (settings version 47)

The first local-only release adds `DeepSeek Harness`, `Kimi Code`,
`Doubao / Ark`, `Zhipu GLM`, `MiniMax`, `Qianfan / ERNIE`, and
`Hunyuan / CodeBuddy`. They share one read-only scanner and show token/request
history without inventing a remaining-quota percentage.

- Kimi Code reads only `sessions/**/agents/main/wire.jsonl`, accepts only
  `usage.record` rows with `agentId = main` and `usageScope = turn`, and ignores
  every `agent-*` sub-agent folder.
- DeepSeek Harness reads the newest canonical `session.vN.jsonl[.zstd]` in each
  session directory. It excludes seeded sessions and records both root and
  delegated Agent sessions in the aggregate usage history. Its fold mirrors
  the official token meter: a later sample for the
  same turn/step replaces the earlier sample, while a retry starts a new
  billable attempt. Harness Agents never become floating-circle targets.
- OpenCode is opened with SQLite read-only/no-mutex flags. Assistant message
  rows are classified by their recorded `providerID` and `modelID`; no cost
  field is required and no credentials are read.
- DeepSeek API keeps the official balance endpoint and also receives local
  DeepSeek-family token history. DeepSeek Harness is a separate transport view
  so users can inspect all models used by Harness without folding transport
  identity into the model provider.
- Model-family views are excluded from the combined total because the same
  request may also appear under OpenCode or Harness. This prevents double
  counting while retaining each provider's own usage card and charts.

Quota rings remain absent for these adapters until a stable, documented quota
source exists. A missing quota is shown as unavailable/local-usage-only, never
as zero remaining.

DeepSeek's balance has no denominator, so the first version deliberately does
not turn it into a progress ring. Users may later set an optional local budget;
that budget must be labelled as user-defined, not provider quota.

## Architecture changes

### Milestone 0: shared contracts

- Add a generic `AccountBalance` snapshot (currency, total, granted,
  topped-up and availability) instead of overloading OpenRouter-specific spend
  fields.
- Add provider capabilities for `quota_windows`, `account_balance`,
  `local_usage`, `manual_secret` and `config_discovery`. Settings can then show
  only operations supported by the selected driver.
- Keep `LimitWindow` plus `additional_limits` as the data-driven quota model.
  Five-hour, weekly and monthly plans map to primary, secondary and an
  additional `monthly` window.
- Preserve the existing compact design: the circle renders no more than two
  selected metrics, while the card is unbounded and data-driven.
- Treat provider identity and model identity separately. A Volcengine plan is
  one provider instance even if the active model changes from Doubao to
  DeepSeek.

### Milestone 1: DeepSeek API balance

- Add the `DeepSeek` provider atomically to the provider enum, registry,
  settings migration, worker dispatch, popup tabs, tray/taskbar metrics,
  caches, icons and all provider combination tests.
- Add `deepseek.rs` with a small HTTPS client for
  `GET https://api.deepseek.com/user/balance`.
- Resolve credentials in this order: protected manual key, `DEEPSEEK_API_KEY`,
  then a supported Qwen Code/OpenCode configuration. Never persist a discovered
  key.
- Parse every returned currency; retain decimal values exactly and avoid binary
  floating-point arithmetic for money.
- Show availability, total balance, granted balance and topped-up balance in
  the detail card. The floating circle shows the DeepSeek mark and a compact
  balance state rather than an artificial ring.
- Handle 401, 403, 429, timeouts and offline cache state separately, with
  bounded retries and no secret-bearing error text.

Acceptance criteria:

- CNY-only, USD-only and multi-currency fixtures render correctly.
- Invalid or expired keys show an actionable state without deleting the key.
- Enabling DeepSeek alone, with each existing provider, and with all providers
  keeps tab/icon identity correct.
- Windows and Linux builds remain Clippy-clean.

### Milestone 2: Qwen Code agent

- Add a `QwenCode` provider that discovers the CLI, `~/.qwen/settings.json`,
  supported `.env` locations and the official monthly token ledger without
  modifying them.
- Detect and label Qwen Plus/Pro OAuth, QwenCloud Token Plan/standard API,
  ModelStudio Coding Plan/Token Plan/standard API and third-party backends as
  distinct account modes.
- Reuse Qwen Code's local usage records for daily/monthly token and request
  views. The official ledger has no cost field, so do not estimate or display
  spend. Do not present `/stats` totals as remaining subscription quota.
- Support multiple isolated Qwen configuration folders only after a reliable
  folder selector is available; otherwise keep one local-history owner to
  prevent double counting, matching the existing OpenCode rule.
- Add an official quota adapter only when a documented endpoint or supported
  local response field is verified. It must be feature-gated so upstream
  changes cannot silently corrupt the display.

Acceptance criteria:

- Every auth mode is recognized without exposing its key or OAuth token.
- Local-history deduplication is stable across rescans and restarts.
- Plus/Pro and API modes are labelled accurately; the retired legacy free OAuth
  mode is not confused with the current subscription flow. Unsupported
  remaining quota is shown as unavailable, not zero.

Implementation status:

- Settings version 44 adds one disabled `QwenCode` instance to existing
  installations.
- The scanner reads only `token-usage-YYYY-MM.jsonl`, accepts schema version 1,
  rejects symlinks and malformed rows, deduplicates by record ID, and keeps a
  365-day local cache plus the recent hourly rollup.
- `QWEN_HOME` and `QWEN_RUNTIME_DIR` follow Qwen Code's documented precedence.
  API keys and OAuth tokens are never persisted, rendered or logged by Minibar.

### Milestone 3: Volcengine Ark and Doubao

- Add a `VolcengineArk` provider for normal Ark API accounts and the two
  subscription plan types. Keep the plan type explicit in settings.
- Discover only documented configurations from supported coding agents. Do not
  call a Coding Plan endpoint from Agent Minibar simply to test the key.
- For a normal Ark API key, add usage/cost data only through documented billing
  APIs. For Coding/Agent Plan, consume quota windows only from an official
  quota API, documented response metadata, or a local agent record.
- Map verified plan windows to 5-hour, weekly and monthly metrics. Default the
  circle to 5-hour plus weekly; let the user replace either with monthly.
- Use the Ark mark for shared-plan identity and show the active model badge
  (`Doubao`, `DeepSeek`, `Kimi`, and so on). A direct Doubao API configuration
  can use the Doubao mark because the provider and model are then aligned.

Acceptance criteria:

- Shared-plan usage is counted once even when several agents use the key.
- Model changes do not create a new quota account or reset local history.
- The app never violates the documented Coding Plan restriction on non-coding
  API use.

## Security and reliability checklist

- Allowlist official HTTPS hosts per driver; reject credentials paired with an
  unexpected host unless the user explicitly configures a custom provider.
- Redact authorization headers, API keys, OAuth tokens, query strings and raw
  provider responses before logging.
- Use short connect/read timeouts, jittered backoff and the existing cached
  snapshot when a provider is offline.
- Store monetary values as fixed decimal strings or integer minor units with an
  explicit currency.
- Version every persisted snapshot and migration; disabling a provider stops
  its worker immediately and removes stale UI data.
- Add sanitized response fixtures and parser tests. Never place real
  credentials or account identifiers in the repository.

## Delivery order

1. Land the generic balance/capability contracts.
2. Ship DeepSeek balance support as the first independently useful provider.
3. Ship Qwen Code discovery and local usage, then add remaining quota only when
   an official source exists.
4. Ship Volcengine Ark/Doubao after the quota acquisition path is confirmed to
   comply with Coding/Agent Plan rules.

## Primary references

- DeepSeek balance API:
  <https://api-docs.deepseek.com/api/get-user-balance/>
- Qwen Code authentication modes:
  <https://github.com/QwenLM/qwen-code/blob/main/docs/users/configuration/auth.md>
- Qwen Code local token ledger schema:
  <https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/services/tokenUsageService.ts>
- Qwen Code runtime storage precedence:
  <https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/config/storage.ts>
- Volcengine Ark Coding Plan overview and restrictions:
  <https://console.volcengine.com/ark/region:cn-beijing/docs/coding-plan-personal-plan-overview>
- Alibaba Cloud ModelStudio Coding Plan restrictions:
  <https://help.aliyun.com/en/model-studio/coding-plan>
