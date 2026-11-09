## Agent Minibar 1.0.0 - Rebuilt from the tray up

![Agent Minibar 1.0 Home](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/home.png)

**New**
- A rewritten popup and Settings window, both rendered natively with GPUI. Smooth page transitions, an acrylic popup and a Mica Settings window.
- Provider instances: add several accounts of any provider, each with its own name, badge color, refresh interval, Home layout and tab.
- Home edit mode: pick Cards, Lines, Rings or Tiles per provider, reorder blocks and choose what shows on Home.

  ![Accounts and Home layouts](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/accounts.webp)

- Animated in-app notifications with sounds. Limit alerts show the quota they are about, and updates report download progress.

  ![Notifications](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/notifications.webp)

- Popup themes: Fluent, Vercel, Apple, and any installed VS Code color theme with contrast and tint controls. Custom accent colors, a font picker and an independent popup border.

  ![Themes](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/themes.webp)

- Localization: English, Русский, Deutsch, Français, Español, Português (Brasil), 日本語 and 简体中文. Auto follows the Windows display language.

  ![Languages](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/languages.webp)

- Redeem banked Codex and Claude resets from the popup, and see confirmed forced resets from the public reset feed.
- Manage OpenRouter keys from the popup's OpenRouter tab.

  ![OpenRouter keys](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/openrouter.webp)

- Claude cloud session credit balances, and renewal warnings before a Claude login expires.

**Improved**
- Usage tab: animated cost and token charts, per-model visibility, spend donut on Home and provider rankings.

  ![Usage](https://raw.githubusercontent.com/Truman-fan/Agent-Minibar/main/docs/media/1.0.0/usage.webp)

- Usage pace indicators on quota cards, and reset countdowns with clearer icons.
- Onboarding previews its choices beside the popup.
- Settings and usage persistence moved off the UI and tray threads; lower idle memory use while scanning usage.
- A live software rendering option for systems where GPU rendering misbehaves.
- Stream Deck keys follow provider instances.

**Fixed**
- Popup cards adapt to long translations; account headings and full dollar amounts no longer clip.
- Claude usage from deleted transcripts is kept, and an inherited `CLAUDE_CONFIG_DIR` is ignored.
- Audit fixes in usage statistics, security and worker lifecycle.

Settings move to the new window automatically; existing profiles migrate to provider instances on first start.
