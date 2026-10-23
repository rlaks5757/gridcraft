//! The app's side of the PPM host lock (the engine's is `gridcraft_engine::hostlock`): the grade
//! of the commands the app handles itself, and `commands.json`, the list the administrator's
//! screen reads. Every command has a grade; the parent's overrides can turn `Off` commands on
//! and `On` commands off, and `Locked` ones never.

use gridcraft_engine::hostlock::{Grade, Policy};

/// The grade of the command `id` (an app command or an engine command); `None` for an id nobody
/// classified. The print dialog's commands are `Off`, switched by `file.print` (see [`enabled`]).
pub fn grade(id: &str) -> Option<Grade> {
    if crate::UI_COMMANDS_OFF.contains(&id) {
        Some(Grade::Off)
    } else if crate::UI_COMMANDS_ALLOWED.contains(&id) || crate::UI_COMMANDS_PLUMBING.contains(&id) {
        Some(Grade::On)
    } else {
        gridcraft_engine::hostlock::grade(id)
    }
}

/// The command whose switch decides `id`: the print dialog's own commands follow `file.print`.
fn switch(id: &str) -> &str {
    if crate::UI_COMMANDS_OFF.contains(&id) { "file.print" } else { id }
}

/// Is `id` on for a session locked with `policy` (`None`: not locked)?
pub fn enabled(policy: Option<&Policy>, id: &str) -> bool {
    // The app's plumbing works whatever the policy says.
    if crate::UI_COMMANDS_PLUMBING.contains(&id) {
        return true;
    }
    // Getting data is one feature with two commands: both must be on (as in the engine).
    if matches!(id, "data.getData" | "data.fromTextCsv") {
        return policy.is_none_or(|p| ["data.getData", "data.fromTextCsv"].iter().all(|c| p.enabled(c, Grade::On)));
    }
    match policy {
        None => true,
        Some(p) => {
            let id = switch(id);
            grade(id).is_some_and(|g| p.enabled(id, g))
        }
    }
}

/// The app commands the administrator can switch: `(id, label, menu location)`.
const APP_COMMANDS: &[(&str, &str, &[&str])] = &[
    ("app.language.english", "Interface Language: English", &["File", "Options"]),
    ("app.language.japanese", "Interface Language: Japanese", &["File", "Options"]),
    ("app.language.set", "Interface Language", &["File", "Options"]),
    ("options.autoComplete", "Enable AutoComplete for cell values", &["File", "Options"]),
    ("view.animations", "Animations", &["File", "Options"]),
    ("view.collapseRibbon", "Collapse Ribbon", &[]),
    ("view.darkMode", "Dark Mode", &[]),
    ("view.interfaceScale", "Interface Size", &["File", "Options"]),
    ("view.theme", "Display Theme", &["File", "Options"]),
];

/// The command list (`ppm/commands.json`, copied to `dist/web` by `ppm/build-web.sh`):
/// `{ app, schema, commands }` with `{ id, label, labelKo, location, grades }` for every command,
/// by id. `labelKo` is null where the app has no Korean text for the label; a label with no
/// letters in it ("100%") reads the same in Korean.
pub fn commands_json() -> String {
    let korean = |label: &str| {
        let ko = crate::i18n::Language::Ko.tr(label);
        (ko != label || !label.chars().any(char::is_alphabetic)).then(|| ko.to_string())
    };
    let entry = |id: &str, label: &str, location: &[&str], grade: Option<Grade>| {
        serde_json::json!({
            "id": id,
            "label": label,
            "labelKo": korean(label),
            "location": location,
            "grades": {"builder": grade.map_or("unclassified", Grade::name)},
        })
    };
    let mut rows: Vec<(String, serde_json::Value)> = Vec::new();
    for c in gridcraft_engine::cmd::command_specs() {
        rows.push((c.id.to_string(), entry(c.id, c.label, c.menu, grade(c.id))));
    }
    // The commands the app runs itself, where the user finds them (File ▸ Options; `[]`: a
    // title-bar switch or a shortcut only). The print dialog's commands follow `file.print` and
    // the plumbing is no switch, so neither is listed.
    for (id, label, location) in APP_COMMANDS {
        rows.push((id.to_string(), entry(id, label, location, grade(id))));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let list: Vec<serde_json::Value> = rows.into_iter().map(|r| r.1).collect();
    serde_json::to_string_pretty(&serde_json::json!({"app": "grid", "schema": 1, "commands": list})).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// `ppm/commands.json` is the table: run with `UPDATE_COMMANDS_JSON=1` to write it again.
    #[test]
    fn commands_json_matches_the_tables() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppm/commands.json");
        let want = commands_json();
        assert!(!want.contains("unclassified"), "a command has no grade");
        if std::env::var_os("UPDATE_COMMANDS_JSON").is_some() {
            std::fs::write(&path, &want).unwrap();
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(have, want, "ppm/commands.json is out of date: run the tests with UPDATE_COMMANDS_JSON=1");
    }

    /// The PPM build starts in Korean, so the administrator screen needs a Korean label for every command.
    #[test]
    fn every_command_has_a_korean_label() {
        let list: serde_json::Value = serde_json::from_str(&commands_json()).unwrap();
        let missing: Vec<&str> = list["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["labelKo"].as_str().is_none_or(str::is_empty))
            .filter_map(|c| c["id"].as_str())
            .collect();
        assert!(missing.is_empty(), "commands without a Korean label: {missing:?}");
    }

    /// The command search shows the menu path a group at a time: every group name has a Korean entry.
    #[test]
    fn every_menu_group_has_a_korean_name() {
        let list: serde_json::Value = serde_json::from_str(&commands_json()).unwrap();
        let mut missing = std::collections::BTreeSet::new();
        for part in list["commands"].as_array().unwrap().iter().flat_map(|c| c["location"].as_array().unwrap()).filter_map(|p| p.as_str()) {
            if crate::i18n::Language::Ko.tr(part) == part {
                missing.insert(part.to_string());
            }
        }
        assert!(missing.is_empty(), "menu names without a Korean entry: {missing:?}");
    }

    #[test]
    fn app_command_lists_are_sorted_and_disjoint() {
        for list in [crate::UI_COMMANDS_OFF, crate::UI_COMMANDS_ALLOWED] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
        }
        assert!(crate::UI_COMMANDS_OFF.iter().all(|id| !crate::UI_COMMANDS_ALLOWED.contains(id)));
        for list in [crate::UI_COMMANDS_PLUMBING, crate::UI_COMMANDS_ALLOWED] {
            assert!(list.windows(2).all(|w| w[0] < w[1]));
        }
        assert!(crate::UI_COMMANDS_PLUMBING.iter().all(|id| !crate::UI_COMMANDS_ALLOWED.contains(id) && !crate::UI_COMMANDS_OFF.contains(id)));
        let listed: Vec<&str> = APP_COMMANDS.iter().map(|c| c.0).collect();
        assert_eq!(listed, crate::UI_COMMANDS_ALLOWED, "every switchable app command has a row, in order");
    }
}
