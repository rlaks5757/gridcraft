//! Whether the page starts GridCraft. The PPM build (`ppm-host` feature) works only inside a
//! same-origin parent page that talks to it; opened directly it shows a notice and nothing else.

use gridcraft_ui_egui::i18n::Language;

/// Shown in place of the app when a PPM build is not framed by the PPM page.
pub const OPEN_FROM_PPM: &str = "<p>PPM에서 열어 주세요</p>";

#[derive(Debug, PartialEq, Eq)]
pub enum Start {
    Run,
    OpenFromPpm,
}

/// `host_build`: made with the `ppm-host` feature. `host_param`: the page's `?host=` value.
/// `has_bridge`: a same-origin parent page was found.
pub fn decide(host_build: bool, host_param: Option<&str>, has_bridge: bool) -> Start {
    if host_build && !(host_param == Some("parent") && has_bridge) { Start::OpenFromPpm } else { Start::Run }
}

/// The interface language the page starts in. The PPM build (`host_build`) always starts in Korean:
/// it embeds a Korean font and ignores the browser's language. The upstream build follows the
/// browser, except a CJK one, since it bundles no CJK font and that interface would render as tofu.
pub fn start_language(host_build: bool, browser: Language) -> Language {
    if host_build {
        Language::Ko
    } else if matches!(browser, Language::Zh | Language::Ja | Language::Ko) {
        Language::default()
    } else {
        browser
    }
}

/// Is the parent on the page's own origin? `parent` is what the page could read of the parent's
/// origin: `None` when the browser refused (a page from another origin).
pub fn same_origin(parent: Option<&str>, own: &str) -> bool {
    parent == Some(own)
}

/// The command overrides in the `policy` of a parent's open message, given as the JSON text of
/// `policy`: `{ "overrides": { "<command id>": true | false } }`. Anything else (not an object,
/// no `overrides`, an array, values that are no boolean) is left out, so a malformed policy changes
/// nothing. There is no cap here: the server limits the count.
pub fn policy_overrides(policy_json: &str) -> Vec<(String, bool)> {
    let Ok(serde_json::Value::Object(policy)) = serde_json::from_str::<serde_json::Value>(policy_json) else { return Vec::new() };
    let Some(serde_json::Value::Object(overrides)) = policy.get("overrides") else { return Vec::new() };
    overrides.iter().filter_map(|(id, on)| Some((id.clone(), on.as_bool()?))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ppm_build_without_the_parent_page_opens_nothing() {
        assert_eq!(decide(true, None, false), Start::OpenFromPpm, "opened directly");
        assert_eq!(decide(true, Some("parent"), false), Start::OpenFromPpm, "no same-origin parent");
        assert_eq!(decide(true, None, true), Start::OpenFromPpm, "framed but not asked for host mode");
        assert_eq!(decide(true, Some("other"), true), Start::OpenFromPpm);
        assert_eq!(decide(true, Some("parent"), true), Start::Run);
    }

    #[test]
    fn the_upstream_build_is_unchanged() {
        for (param, bridge) in [(None, false), (Some("parent"), false), (Some("parent"), true), (None, true)] {
            assert_eq!(decide(false, param, bridge), Start::Run);
        }
    }

    #[test]
    fn the_ppm_build_starts_in_korean_whatever_the_browser_says() {
        for browser in Language::ALL {
            assert_eq!(start_language(true, browser), Language::Ko);
        }
    }

    #[test]
    fn the_upstream_build_follows_the_browser_except_cjk() {
        assert_eq!(start_language(false, Language::De), Language::De);
        assert_eq!(start_language(false, Language::En), Language::En);
        for cjk in [Language::Ko, Language::Ja, Language::Zh] {
            assert_eq!(start_language(false, cjk), Language::En, "no CJK font in the upstream web build");
        }
    }

    #[test]
    fn only_a_parent_on_the_same_origin_counts() {
        assert!(same_origin(Some("https://ppm.example"), "https://ppm.example"));
        assert!(!same_origin(Some("https://evil.example"), "https://ppm.example"));
        assert!(!same_origin(None, "https://ppm.example"), "the browser refused to say");
    }

    #[test]
    fn overrides_keep_only_boolean_values_under_a_well_formed_policy() {
        let mut got = policy_overrides(r#"{"overrides": {"file.print": true, "home.bold": false, "a": "yes", "b": 1, "c": null, "d": {}}}"#);
        got.sort();
        assert_eq!(got, vec![("file.print".to_string(), true), ("home.bold".to_string(), false)]);
        for bad in
            ["", "null", "[]", "5", "{}", r#"{"overrides": null}"#, r#"{"overrides": []}"#, r#"{"overrides": "x"}"#, r#"{"other": {"a": true}}"#, "{"]
        {
            assert!(policy_overrides(bad).is_empty(), "{bad}");
        }
    }

    #[test]
    fn four_thousand_overrides_are_all_taken() {
        let body: Vec<String> = (0..4000).map(|i| format!("\"cmd.{i}\": true")).collect();
        assert_eq!(policy_overrides(&format!("{{\"overrides\": {{{}}}}}", body.join(","))).len(), 4000);
    }

    #[test]
    fn the_notice_says_to_open_from_ppm() {
        assert!(OPEN_FROM_PPM.contains("PPM에서 열어 주세요"));
    }
}
