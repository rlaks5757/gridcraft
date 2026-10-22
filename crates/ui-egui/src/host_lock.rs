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
    } else if crate::UI_COMMANDS_ALLOWED.contains(&id) {
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
    match policy {
        None => true,
        Some(p) => {
            let id = switch(id);
            grade(id).is_some_and(|g| p.enabled(id, g))
        }
    }
}

/// The command list (`ppm/commands.json`, copied to `dist/web` by `ppm/build-web.sh`):
/// `{ app, schema, commands }` with `{ id, label, labelKo, location, grades }` for every command,
/// by id. `labelKo` is null where the app has no Korean text for the label.
pub fn commands_json() -> String {
    let korean = |label: &str| {
        let ko = crate::i18n::Language::Ko.tr(label);
        (ko != label).then(|| ko.to_string())
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
    // An app command is listed under the tab or menu its id starts with (`view.theme` → View).
    // (The print dialog's commands follow `file.print` and are not listed.)
    for id in crate::UI_COMMANDS_ALLOWED {
        if rows.iter().all(|r| r.0 != *id) {
            let head = id.split('.').next().unwrap_or(id);
            let area = head[..1].to_uppercase() + &head[1..];
            rows.push((id.to_string(), entry(id, id, &[area.as_str()], grade(id))));
        }
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

    #[test]
    fn app_command_lists_are_sorted_and_disjoint() {
        for list in [crate::UI_COMMANDS_OFF, crate::UI_COMMANDS_ALLOWED] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
        }
        assert!(crate::UI_COMMANDS_OFF.iter().all(|id| !crate::UI_COMMANDS_ALLOWED.contains(id)));
    }
}
