use std::collections::BTreeMap;

use serde_yaml_ng::{Mapping, Value};
use tauri::Url;

pub const SLOVE_DELAY_DISPLAY_PERCENT: u16 = 40;
pub const DEFAULT_DELAY_DISPLAY_PERCENT: u16 = 200;

const SLOVE_PATH_FRAGMENT: &str = "/slove/";
const TIMEOUT_DELAY: u16 = 10_000;

/// Resolves the display-only percentage for a node from its real profile/provider URLs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DelayDisplayContext {
    profile_url: Option<String>,
    provider_urls: BTreeMap<String, String>,
}

impl DelayDisplayContext {
    pub fn from_runtime(profile_url: Option<&str>, runtime: Option<&Mapping>) -> Self {
        let provider_urls = runtime
            .and_then(|config| config.get("proxy-providers"))
            .and_then(Value::as_mapping)
            .map(|providers| {
                providers
                    .iter()
                    .filter_map(|(name, provider)| {
                        let name = name.as_str()?;
                        let url = provider.get("url").and_then(Value::as_str)?;
                        Some((name.to_owned(), url.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            profile_url: profile_url.map(str::to_owned),
            provider_urls,
        }
    }

    pub fn node_percent(&self, provider_name: Option<&str>) -> u16 {
        display_percent_for_urls([
            self.profile_url.as_deref(),
            provider_name.and_then(|name| self.provider_urls.get(name).map(String::as_str)),
        ])
    }

    pub fn profile_percent(&self) -> u16 {
        display_percent_for_urls([self.profile_url.as_deref()])
    }
}

/// Matches the literal path fragment only; hosts, queries, and `/slove` without a trailing slash do not count.
pub fn display_percent_for_url(url: &str) -> u16 {
    let Ok(url) = Url::parse(url) else {
        return DEFAULT_DELAY_DISPLAY_PERCENT;
    };

    if url.path().contains(SLOVE_PATH_FRAGMENT) {
        SLOVE_DELAY_DISPLAY_PERCENT
    } else {
        DEFAULT_DELAY_DISPLAY_PERCENT
    }
}

fn display_percent_for_urls<'a>(urls: impl IntoIterator<Item = Option<&'a str>>) -> u16 {
    if urls
        .into_iter()
        .flatten()
        .any(|url| display_percent_for_url(url) == SLOVE_DELAY_DISPLAY_PERCENT)
    {
        SLOVE_DELAY_DISPLAY_PERCENT
    } else {
        DEFAULT_DELAY_DISPLAY_PERCENT
    }
}

/// Applies the display percentage to measured values only; core sentinels remain unchanged.
pub fn display_delay(delay: u16, percent: u16) -> u16 {
    if delay == 0 || delay >= TIMEOUT_DELAY {
        return delay;
    }

    let scaled = (u32::from(delay) * u32::from(percent) + 50) / 100;
    u16::try_from(scaled.max(1)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_DELAY_DISPLAY_PERCENT, DelayDisplayContext, SLOVE_DELAY_DISPLAY_PERCENT, display_delay,
        display_percent_for_url,
    };

    #[test]
    fn path_fragment_matching_is_literal_and_url_only() {
        assert_eq!(
            display_percent_for_url("https://example.com/slove/subscription"),
            SLOVE_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            display_percent_for_url("https://example.com/prefix/slove/file"),
            SLOVE_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            display_percent_for_url("https://slove.example.com/profile"),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            display_percent_for_url("https://example.com/api?next=/slove/profile"),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            display_percent_for_url("https://example.com/slove"),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            display_percent_for_url("https://example.com/SLOVE/profile"),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(display_percent_for_url("not a url"), DEFAULT_DELAY_DISPLAY_PERCENT);
    }

    #[test]
    fn provider_and_profile_sources_are_combined() {
        let runtime: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxy-providers:
  slove-provider:
    url: https://cdn.example.com/slove/provider.yaml
  other-provider:
    url: https://cdn.example.com/other/provider.yaml
"#,
        )
        .expect("parse runtime providers");
        let context = DelayDisplayContext::from_runtime(Some("https://example.com/profile.yaml"), Some(&runtime));

        assert_eq!(context.node_percent(None), DEFAULT_DELAY_DISPLAY_PERCENT);
        assert_eq!(
            context.node_percent(Some("slove-provider")),
            SLOVE_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            context.node_percent(Some("other-provider")),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            context.node_percent(Some("missing-provider")),
            DEFAULT_DELAY_DISPLAY_PERCENT
        );

        let slove_profile = DelayDisplayContext::from_runtime(Some("https://example.com/slove/profile"), None);
        assert_eq!(
            slove_profile.node_percent(Some("other-provider")),
            SLOVE_DELAY_DISPLAY_PERCENT
        );
    }

    #[test]
    fn display_rounding_preserves_sentinels_and_never_rounds_to_zero() {
        assert_eq!(display_delay(1, 40), 1);
        assert_eq!(display_delay(2, 40), 1);
        assert_eq!(display_delay(3, 40), 1);
        assert_eq!(display_delay(4, 40), 2);
        assert_eq!(display_delay(101, 40), 40);
        assert_eq!(display_delay(102, 40), 41);
        assert_eq!(display_delay(150, 200), 300);
        assert_eq!(display_delay(0, 40), 0);
        assert_eq!(display_delay(10_000, 40), 10_000);
        assert_eq!(display_delay(u16::MAX, 200), u16::MAX);
    }
}
