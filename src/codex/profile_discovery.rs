//! Detects the Windows desktop's standard profile and an isolated native CLI profile.

use std::path::{Path, PathBuf};

use super::{decode_id_token, load_auth_tokens, non_empty};
use crate::{
    discovery::{self, CandidateSource},
    instances::folder_identity,
    settings::{CodexProfilePair, InstanceSource, ProviderKind, Settings},
};

pub(crate) fn discover(settings: &mut Settings) -> bool {
    let Some(cli) = std::env::var_os("CODEX_HOME").map(PathBuf::from) else {
        return false;
    };
    let Some(dirs) = directories::BaseDirs::new() else {
        return false;
    };
    let desktop = dirs.home_dir().join(".codex");
    if !cli.is_absolute()
        || !desktop.join(".codex-global-state.json").is_file()
        || cli.join(".codex-global-state.json").is_file()
        || !separate_logins(&desktop, &cli)
    {
        return false;
    }
    let candidates = discovery::discover(None);
    if !candidates
        .iter()
        .any(|c| c.source == CandidateSource::DesktopApp)
        || !candidates
            .iter()
            .any(|c| c.source != CandidateSource::DesktopApp)
    {
        return false;
    }
    register_pair(settings, desktop, cli)
}

#[derive(PartialEq, Eq)]
struct LoginIdentity {
    account_id: String,
    subject: Option<String>,
}

impl LoginIdentity {
    fn read(home: &Path) -> Option<Self> {
        let tokens = load_auth_tokens(&home.join("auth.json")).ok()?;
        non_empty(tokens.access_token)?;
        Some(Self {
            account_id: non_empty(tokens.account_id)?,
            subject: tokens
                .id_token
                .as_deref()
                .and_then(decode_id_token)
                .and_then(|claims| non_empty(claims.sub)),
        })
    }

    fn is_distinct_from(&self, other: &Self) -> bool {
        self.account_id != other.account_id
            || matches!((&self.subject, &other.subject), (Some(a), Some(b)) if a != b)
    }
}

fn separate_logins(desktop: &Path, cli: &Path) -> bool {
    if folder_identity(Some(desktop), ProviderKind::Codex)
        == folder_identity(Some(cli), ProviderKind::Codex)
    {
        return false;
    }
    LoginIdentity::read(desktop)
        .zip(LoginIdentity::read(cli))
        .is_some_and(|(a, b)| a.is_distinct_from(&b))
}

fn register_pair(settings: &mut Settings, desktop: PathBuf, cli: PathBuf) -> bool {
    if settings.codex_profile_pair.is_some() {
        return false;
    }
    let kind = ProviderKind::Codex;
    let mut codex = settings
        .instances
        .iter_mut()
        .filter(|item| item.driver == kind);
    let Some(ambient) = codex.next() else {
        return false;
    };
    if codex.next().is_some()
        || !ambient.is_primary()
        || ambient.config_folder().is_some()
        || (!ambient.enabled && settings.onboarding_completed)
    {
        return false;
    }

    // The inherited override was already the legacy provider's home. Keep
    // its id and history with that profile when installing the second one.
    ambient.source = InstanceSource::ConfigFolder { path: Some(cli) };
    if ambient.name == kind.display_name() {
        ambient.name = crate::i18n::tr("codex-cli").into();
    }
    if ambient.badge.is_empty() {
        ambient.badge = "CLI".into();
    }
    let cli_id = ambient.id.clone();
    let mut desktop_instance =
        crate::instances::ProviderInstance::new(kind, crate::i18n::tr("codex-desktop-app"));
    desktop_instance.enabled = ambient.enabled;
    desktop_instance.badge = "APP".into();
    desktop_instance.source = InstanceSource::ConfigFolder {
        path: Some(desktop),
    };
    desktop_instance.in_usage_overview = false;
    let desktop_id = desktop_instance.id.clone();
    settings.add_instance(desktop_instance);
    settings.codex_profile_pair = Some(CodexProfilePair { desktop_id, cli_id });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_copies_of_the_same_login_are_not_two_accounts() {
        let a = LoginIdentity {
            account_id: "workspace".into(),
            subject: Some("user-a".into()),
        };
        let b = LoginIdentity {
            account_id: "workspace".into(),
            subject: None,
        };
        assert!(!a.is_distinct_from(&a));
        assert!(!a.is_distinct_from(&b));
        assert!(a.is_distinct_from(&LoginIdentity {
            subject: Some("user-b".into()),
            ..b
        }));
    }

    #[test]
    fn discovery_requires_two_readable_chatgpt_logins_in_separate_folders() {
        let desktop = tempfile::tempdir().unwrap();
        let cli = tempfile::tempdir().unwrap();
        let write_auth = |home: &Path, body: &[u8]| {
            std::fs::write(home.join("auth.json"), body).unwrap();
        };
        write_auth(
            desktop.path(),
            br#"{"tokens":{"account_id":"desktop","access_token":"a"}}"#,
        );
        assert!(!separate_logins(desktop.path(), desktop.path()));
        assert!(!separate_logins(desktop.path(), cli.path()));
        write_auth(cli.path(), br#"{"OPENAI_API_KEY":"synthetic"}"#);
        assert!(!separate_logins(desktop.path(), cli.path()));
        write_auth(
            cli.path(),
            br#"{"tokens":{"account_id":"desktop","access_token":"b"}}"#,
        );
        assert!(!separate_logins(desktop.path(), cli.path()));
        write_auth(
            cli.path(),
            br#"{"tokens":{"account_id":"cli","access_token":"b"}}"#,
        );
        assert!(separate_logins(desktop.path(), cli.path()));
    }

    #[test]
    fn detected_codex_profiles_keep_existing_id_and_prefer_desktop_for_automatic_circle() {
        let mut settings = Settings::default();
        let cli = crate::instances::ProviderId::primary(ProviderKind::Codex);
        settings.instance_mut(cli).unwrap().enabled = true;
        let desktop_home = PathBuf::from(r"C:\profiles\desktop");
        let cli_home = PathBuf::from(r"D:\profiles\cli");
        assert!(register_pair(
            &mut settings,
            desktop_home.clone(),
            cli_home.clone()
        ));
        let desktop = settings.floating_circle_target(0).unwrap();
        assert_ne!(desktop, cli);
        assert_eq!(
            settings.instance(cli).unwrap().config_folder(),
            Some(cli_home.clone())
        );
        assert_eq!(
            settings.instance(desktop).unwrap().config_folder(),
            Some(desktop_home.clone())
        );
        assert_eq!(settings.floating_circle_slots.len(), 1);
        settings.set_second_floating_circle(true);
        assert_eq!(settings.floating_circle_target(1), Some(cli));
        settings.set_floating_circle_provider(0, cli);
        assert_eq!(settings.floating_circle_target(0), Some(cli));
        settings.remove_instance(desktop);
        assert!(!register_pair(&mut settings, desktop_home, cli_home));
    }
}
