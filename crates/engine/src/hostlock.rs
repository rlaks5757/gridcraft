//! Every command has a grade. `Locked`: never runs in a locked session, whatever the policy
//! says (opening, creating and closing workbooks, scripts and macros; the parent's own workbook
//! arrives as bytes through `file.open`). `Off`: off until the parent's policy turns it on (Save
//! As, exports, printing, opening links). `On`: on until the policy turns it off. Edits inside
//! the workbook are `On`, including the ones that read a local picture or data file to put in
//! it. Save hands the workbook (xlsx) to the parent.
//!
//! The parent sends `policy: { overrides: { "<command id>": true | false } }` with the workbook
//! ([`Policy`]); only commands that differ from their grade are listed. A command nobody
//! classified is refused, and the drift test below fails until it is classified.

use serde_json::Value as Json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// True in builds made with the `ppm-host` feature (`ppm/build-web.sh`): every session is locked
/// and nothing at run time can unlock it.
pub const HOST_BUILD: bool = cfg!(feature = "ppm-host");

/// Text every host-lock refusal carries, so the UI can drop it without a message.
pub const BLOCKED_TEXT: &str = "blocked by the PPM host lock";

/// Whether a command runs by default and what a policy can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Locked,
    Off,
    On,
}

impl Grade {
    /// The name `commands.json` uses.
    pub fn name(self) -> &'static str {
        match self {
            Grade::Locked => "locked",
            Grade::Off => "off",
            Grade::On => "on",
        }
    }
}

/// The parent's overrides. Cloned handles share the state, so a service made once at start sees
/// a policy that arrives with the next open message.
#[derive(Clone, Debug, Default)]
pub struct Policy(Arc<Mutex<HashMap<String, bool>>>);

impl Policy {
    /// Is a command of `grade` on? `true` turns an `Off` command on, `false` turns an `On`
    /// command off; a `Locked` command never is, and an id nobody overrode keeps its grade.
    pub fn enabled(&self, id: &str, grade: Grade) -> bool {
        let over = self.0.lock().unwrap_or_else(|e| e.into_inner()).get(id).copied();
        match grade {
            Grade::Locked => false,
            Grade::Off => over == Some(true),
            Grade::On => over != Some(false),
        }
    }

    /// Replaces the overrides. Ids that are no command are never asked for, so they do nothing.
    pub fn set(&self, overrides: &[(String, bool)]) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = overrides.iter().cloned().collect();
    }
}

/// Commands a locked session runs as they are.
pub const ALLOWED: &[&str] = &[
    "app.commands",
    "arrange.align",
    "arrange.bringForward",
    "arrange.group",
    "arrange.rotate",
    "arrange.selectionPane",
    "arrange.sendBackward",
    "cell.get",
    "cell.set",
    "cell.toggleCheckbox",
    "chart.addElement",
    "chart.changeColors",
    "chart.changeType",
    "chart.delete",
    "chart.formatSelection",
    "chart.move",
    "chart.quickLayout",
    "chart.selectData",
    "chart.set",
    "chart.shapeFill",
    "chart.shapeOutline",
    "chart.switchRowColumn",
    "data.advancedFilter",
    "data.analysisToolPak",
    "data.circleInvalid",
    "data.clearFilter",
    "data.consolidate",
    "data.dataTable",
    "data.filter",
    "data.filterBy",
    "data.flashFill",
    "data.forecastSheet",
    "data.fromTableRange",
    "data.goalSeek",
    "data.group",
    "data.hideDetail",
    "data.queriesConnections",
    "data.reapply",
    "data.refreshAll",
    "data.removeDuplicates",
    "data.scenarioAdd",
    "data.scenarioDelete",
    "data.scenarioManager",
    "data.scenarioShow",
    "data.scenarioSummary",
    "data.showDetail",
    "data.showLevels",
    "data.solver",
    "data.solverReport",
    "data.sort",
    "data.sortAscending",
    "data.sortDescending",
    "data.subtotal",
    "data.textToColumns",
    "data.ungroup",
    "data.validate",
    "data.validation",
    "document.inspect",
    "draw.eraser",
    "draw.inkToShape",
    "draw.lasso",
    "draw.pen",
    "draw.stroke",
    "edit.autoFill",
    "edit.beginEdit",
    "edit.clearAll",
    "edit.clearClipboard",
    "edit.clearComments",
    "edit.clearContents",
    "edit.clearFormats",
    "edit.clearHyperlinks",
    "edit.copy",
    "edit.cut",
    "edit.fillDown",
    "edit.fillLeft",
    "edit.fillRight",
    "edit.fillSeries",
    "edit.fillUp",
    "edit.find",
    "edit.flashFill",
    "edit.formatPainter",
    "edit.goTo",
    "edit.goToSpecial",
    "edit.paste",
    "edit.pasteSpecial",
    "edit.redo",
    "edit.replace",
    "edit.selectAll",
    "edit.undo",
    "file.options",
    "file.printPreview", // only measures pages (Page Layout and Page Break views); nothing is printed or saved
    "file.properties",
    "formulas.autoSum",
    "formulas.calculateNow",
    "formulas.calculateSheet",
    "formulas.calculationOptions",
    "formulas.createFromSelection",
    "formulas.dateTime",
    "formulas.defineName",
    "formulas.deleteName",
    "formulas.errorChecking",
    "formulas.evaluate",
    "formulas.evaluateFormula",
    "formulas.financial",
    "formulas.functions",
    "formulas.insertFunction",
    "formulas.logical",
    "formulas.lookupReference",
    "formulas.mathTrig",
    "formulas.moreFunctions",
    "formulas.nameManager",
    "formulas.recentlyUsed",
    "formulas.removeArrows",
    "formulas.showFormulas",
    "formulas.text",
    "formulas.traceDependents",
    "formulas.tracePrecedents",
    "formulas.useInFormula",
    "formulas.watchWindow",
    "history.list",
    "home.accounting",
    "home.addIns",
    "home.alignBottom",
    "home.alignCenter",
    "home.alignLeft",
    "home.alignMiddle",
    "home.alignRight",
    "home.alignTop",
    "home.autofitColumnWidth",
    "home.autofitRowHeight",
    "home.bold",
    "home.borders",
    "home.cellStyle",
    "home.clearRules",
    "home.columnWidth",
    "home.comma",
    "home.conditionalFormat",
    "home.decreaseDecimal",
    "home.decreaseFontSize",
    "home.decreaseIndent",
    "home.defaultWidth",
    "home.deleteCells",
    "home.deleteColumns",
    "home.deleteRows",
    "home.deleteSheet",
    "home.doubleUnderline",
    "home.drawBorder",
    "home.fillColor",
    "home.fontColor",
    "home.fontName",
    "home.fontSize",
    "home.formatAsTable",
    "home.formatCells",
    "home.hideColumns",
    "home.hideRows",
    "home.increaseDecimal",
    "home.increaseFontSize",
    "home.increaseIndent",
    "home.insertCells",
    "home.insertColumns",
    "home.insertCopiedCells",
    "home.insertRows",
    "home.insertSheet",
    "home.italic",
    "home.justify",
    "home.lockCell",
    "home.manageRules",
    "home.mergeAcross",
    "home.mergeCells",
    "home.mergeCenter",
    "home.newCellStyle",
    "home.numberFormat",
    "home.orientation",
    "home.percent",
    "home.rowHeight",
    "home.shrinkToFit",
    "home.strikethrough",
    "home.subscript",
    "home.superscript",
    "home.underline",
    "home.unhideColumns",
    "home.unhideRows",
    "home.unmergeCells",
    "home.wrapText",
    "insert.chart",
    "insert.chartRecommendations",
    "insert.checkbox",
    "insert.icons",
    "insert.link",
    "insert.picture",
    "insert.pivotChart",
    "insert.pivotTable",
    "insert.recommendedCharts",
    "insert.recommendedPivotTables",
    "insert.shape",
    "insert.slicer",
    "insert.sparkline",
    "insert.symbol",
    "insert.table",
    "insert.textBox",
    "object.delete",
    "object.move",
    "object.setAnchorMode",
    "pageLayout.background",
    "pageLayout.breaks",
    "pageLayout.headerFooter",
    "pageLayout.margins",
    "pageLayout.orientation",
    "pageLayout.pageSetup",
    "pageLayout.printArea",
    "pageLayout.printGridlines",
    "pageLayout.printHeadings",
    "pageLayout.printTitles",
    "pageLayout.scaleToFit",
    "pageLayout.size",
    "pageLayout.theme",
    "pageLayout.themeColors",
    "pageLayout.themeEffects",
    "pageLayout.themeFonts",
    "pivot.addField",
    "pivot.calculatedField",
    "pivot.changeSource",
    "pivot.collapse",
    "pivot.delete",
    "pivot.fieldList",
    "pivot.fields",
    "pivot.filter",
    "pivot.grandTotals",
    "pivot.group",
    "pivot.layout",
    "pivot.list",
    "pivot.moveField",
    "pivot.refresh",
    "pivot.removeField",
    "pivot.reportLayout",
    "pivot.sort",
    "pivot.style",
    "pivot.subtotals",
    "pivot.valueSettings",
    "range.fill",
    "range.setValues",
    "review.addToDictionary",
    "review.allowEditRanges",
    "review.changeSpelling",
    "review.checkAccessibility",
    "review.convertToComments",
    "review.deleteComment",
    "review.newComment",
    "review.newNote",
    "review.nextComment",
    "review.previousComment",
    "review.protectSheet",
    "review.protectWorkbook",
    "review.replyComment",
    "review.resolveComment",
    "review.showAllNotes",
    "review.showComments",
    "review.showNote",
    "review.spelling",
    "review.thesaurus",
    "review.unprotectSheet",
    "review.workbookStatistics",
    "selection.column",
    "selection.currentRegion",
    "selection.move",
    "selection.next",
    "selection.row",
    "selection.set",
    "selection.stats",
    "shape.setText",
    "sheet.activate",
    "sheet.hide",
    "sheet.move",
    "sheet.moveColumns",
    "sheet.next",
    "sheet.previous",
    "sheet.read",
    "sheet.rename",
    "sheet.tabColor",
    "sheet.unhide",
    "slicer.clear",
    "slicer.delete",
    "slicer.selectAll",
    "slicer.toggle",
    "table.bandedColumns",
    "table.bandedRows",
    "table.convertToRange",
    "table.firstColumn",
    "table.headerRow",
    "table.insertSlicer",
    "table.lastColumn",
    "table.removeDuplicates",
    "table.rename",
    "table.resize",
    "table.style",
    "table.summarizePivot",
    "table.totalFunction",
    "table.totalRow",
    "view.arrangeAll",
    "view.customViews",
    "view.formulaBar",
    "view.freezeFirstColumn",
    "view.freezePanes",
    "view.freezeTopRow",
    "view.gridlines",
    "view.headings",
    "view.hideWindow",
    "view.newWindow",
    "view.normal",
    "view.pageBreakPreview",
    "view.pageLayout",
    "view.rightToLeft",
    "view.ruler",
    "view.showZeros",
    "view.split",
    "view.switchWindows",
    "view.unfreezePanes",
    "view.unhideWindow",
    "view.zoom",
    "view.zoom100",
    "view.zoomToSelection",
    "window.activate",
];
/// Commands a locked session never runs.
pub const BLOCKED: &[&str] = &[
    "automate.allScripts",
    "automate.deleteScript",
    "automate.newScript",
    "automate.recordActions",
    "automate.runScript",
    "file.close",
    "file.new",
    "view.macros",
];
/// Commands that are off until the policy turns them on.
pub const OFF: &[&str] = &[
    "file.exportCsv",
    "file.exportHtml",
    "file.exportPdf",
    "file.print",
    "file.saveAs",
    "link.open",
];
/// Commands a locked session runs only with certain params (see [`permits`]).
pub const CONDITIONAL: &[&str] = &[
    "data.fromTextCsv",
    "data.getData",
    "file.open",
    "file.save",
    "file.saveBytes",
];

/// The grade of the engine command `id`; `None` for an id nobody classified. `file.open` is
/// `Locked`, but see [`permits`].
pub fn grade(id: &str) -> Option<Grade> {
    if BLOCKED.contains(&id) || id == "file.open" {
        Some(Grade::Locked)
    } else if OFF.contains(&id) {
        Some(Grade::Off)
    } else if ALLOWED.contains(&id) || CONDITIONAL.contains(&id) {
        Some(Grade::On)
    } else {
        None
    }
}

/// The command that decides whether `file.saveBytes` may produce `format`: the workbook always,
/// the others as their export or Save As.
fn export_command(format: &str) -> Option<&'static str> {
    match format.to_ascii_lowercase().as_str() {
        "xlsx" => None,
        "csv" | "tsv" => Some("file.exportCsv"),
        "html" => Some("file.exportHtml"),
        "pdf" => Some("file.exportPdf"),
        _ => Some("file.saveAs"),
    }
}

/// The two commands that bring data in are one feature (the same code runs both): switching
/// either one off turns both off.
fn data_import_on(policy: &Policy) -> bool {
    policy.enabled("data.getData", Grade::On) && policy.enabled("data.fromTextCsv", Grade::On)
}

/// Whether a session locked with `policy` (`None`: not locked) may run `id` with `params`.
/// Unclassified ids are refused.
pub fn permits(policy: Option<&Policy>, id: &str, params: &Json) -> bool {
    let Some(policy) = policy else { return true };
    let has = |key: &str| params.get(key).is_some_and(|v| !v.is_null());
    match id {
        // A workbook the parent sent arrives as bytes; no path, no picker.
        "file.open" => has("base64") && !has("path"),
        // Data goes into the open workbook as text the app read; a path would read a local file.
        "data.fromTextCsv" | "data.getData" => data_import_on(policy) && !has("path"),
        // Save hands the xlsx to the parent; a path would pick a file (and its extension a format).
        "file.save" => policy.enabled(id, Grade::On) && (!has("path") || policy.enabled("file.saveAs", Grade::Off)),
        "file.saveBytes" => {
            policy.enabled(id, Grade::On) && params.get("format").and_then(Json::as_str).and_then(export_command).is_none_or(|c| policy.enabled(c, Grade::Off))
        }
        _ => grade(id).is_some_and(|g| policy.enabled(id, g)),
    }
}

/// Whether a locked session lists `id` (command search, menus built from the command table).
pub fn listed(policy: Option<&Policy>, id: &str) -> bool {
    match policy {
        None => true,
        Some(p) => permits(Some(p), id, &Json::Null) && id != "file.open",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use crate::cmd::command_specs;
    use serde_json::json;

    fn locked() -> Session {
        let mut s = Session::new();
        s.lock_host();
        s.new_workbook();
        s
    }

    #[test]
    fn every_command_is_classified_exactly_once() {
        for spec in command_specs() {
            let n = [ALLOWED, BLOCKED, OFF, CONDITIONAL].iter().filter(|l| l.contains(&spec.id)).count();
            assert_eq!(n, 1, "command `{}` must be in exactly one of ALLOWED, BLOCKED, OFF, CONDITIONAL in hostlock.rs", spec.id);
        }
    }

    #[test]
    fn lists_hold_no_stale_ids_and_are_sorted() {
        for list in [ALLOWED, BLOCKED, OFF, CONDITIONAL] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
            for id in list {
                assert!(command_specs().iter().any(|c| c.id == *id), "`{id}` is classified but no such command exists");
            }
        }
    }

    #[test]
    fn locked_session_refuses_every_blocked_and_off_command() {
        let mut s = locked();
        for id in BLOCKED.iter().chain(OFF) {
            let e = s.execute(id, json!({})).unwrap_err().to_string();
            assert!(e.contains(BLOCKED_TEXT), "`{id}` ran in a locked session: {e}");
        }
    }

    #[test]
    fn overrides_turn_off_commands_on_and_on_commands_off() {
        let mut s = locked();
        let policy = s.host_policy().unwrap();
        let refused = |s: &mut Session, id: &str| s.execute(id, json!({})).err().is_some_and(|e| e.to_string().contains(BLOCKED_TEXT));
        assert!(refused(&mut s, "file.exportCsv"));
        policy.set(&[("file.exportCsv".into(), true), ("home.bold".into(), false), ("no.such".into(), true)]);
        assert!(!refused(&mut s, "file.exportCsv"), "an Off command turns on");
        assert!(refused(&mut s, "home.bold"), "an On command turns off");
        assert!(!refused(&mut s, "home.italic"), "the rest is untouched");
        policy.set(&[]);
        assert!(refused(&mut s, "file.exportCsv"));
        assert!(!refused(&mut s, "home.bold"));
    }

    #[test]
    fn locked_commands_stay_locked_whatever_the_policy_says() {
        let mut s = locked();
        let all: Vec<(String, bool)> = BLOCKED.iter().chain(&["file.open"]).map(|id| (id.to_string(), true)).collect();
        s.set_policy(&all);
        for id in BLOCKED {
            assert!(s.execute(id, json!({})).unwrap_err().to_string().contains(BLOCKED_TEXT), "{id}");
        }
        assert!(s.execute("file.open", json!({"path": "/etc/hosts"})).unwrap_err().to_string().contains(BLOCKED_TEXT));
    }

    #[test]
    fn in_workbook_edits_stay_on() {
        let mut s = locked();
        for id in ["insert.picture", "pageLayout.background", "home.addIns", "file.printPreview"] {
            assert!(permits(s.host_policy().as_ref(), id, &json!({})), "{id}");
        }
        assert!(s.execute("home.addIns", json!({})).is_ok());
    }

    #[test]
    fn data_comes_in_as_text_not_as_a_path() {
        let mut s = locked();
        let before = s.documents().len();
        s.execute("data.getData", json!({"text": "a,b\n1,2\n"})).unwrap();
        assert_eq!(s.documents().len(), before, "the data goes into the open workbook");
        for id in ["data.getData", "data.fromTextCsv"] {
            let e = s.execute(id, json!({"path": "/etc/hosts"})).unwrap_err().to_string();
            assert!(e.contains(BLOCKED_TEXT), "{id}");
        }
        for off in ["data.getData", "data.fromTextCsv"] {
            s.set_policy(&[(off.to_string(), false)]);
            for id in ["data.getData", "data.fromTextCsv"] {
                assert!(s.execute(id, json!({"text": "a\n"})).unwrap_err().to_string().contains(BLOCKED_TEXT), "{id} with {off} off");
            }
        }
    }

    #[test]
    fn unlocked_session_is_unchanged() {
        let mut s = Session::new();
        s.new_workbook();
        let e = s.execute("file.exportCsv", json!({})).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(!e.contains(BLOCKED_TEXT));
    }

    #[test]
    fn locked_session_still_edits() {
        let mut s = locked();
        s.execute("cell.set", json!({"ref": "A1", "value": "x"})).unwrap();
        assert!(s.execute("home.bold", json!({})).is_ok());
    }

    #[test]
    fn open_takes_only_bytes_from_the_parent() {
        let mut s = locked();
        let e = s.execute("file.open", json!({"path": "/etc/hosts"})).unwrap_err().to_string();
        assert!(e.contains(BLOCKED_TEXT));
        assert!(s.execute("file.open", json!({})).unwrap_err().to_string().contains(BLOCKED_TEXT));
        let p = s.host_policy();
        assert!(!permits(p.as_ref(), "file.open", &json!({"path": "a.xlsx", "base64": "AA=="})));
        assert!(permits(p.as_ref(), "file.open", &json!({"name": "a.xlsx", "base64": "AA=="})));
    }

    #[test]
    fn save_is_xlsx_bytes_only_until_the_policy_says_otherwise() {
        let mut s = locked();
        assert!(s.execute("file.saveBytes", json!({"format": "xlsx"})).is_ok());
        for format in ["csv", "html", "json", "tsv"] {
            let e = s.execute("file.saveBytes", json!({"format": format})).unwrap_err().to_string();
            assert!(e.contains(BLOCKED_TEXT), "{format}");
        }
        let p = s.host_policy();
        assert!(!permits(p.as_ref(), "file.save", &json!({"path": "x.pdf"})));
        assert!(permits(p.as_ref(), "file.save", &json!({})));
        s.set_policy(&[("file.exportCsv".to_string(), true), ("file.saveAs".to_string(), true)]);
        assert!(s.execute("file.saveBytes", json!({"format": "csv"})).is_ok());
        assert!(s.execute("file.saveBytes", json!({"format": "json"})).is_ok());
        assert!(s.execute("file.saveBytes", json!({"format": "html"})).is_err(), "html is its own export");
        assert!(permits(s.host_policy().as_ref(), "file.save", &json!({"path": "x.xlsx"})));
    }

    #[test]
    fn locked_commands_are_not_listed() {
        let mut s = locked();
        let ids = |s: &Session| s.commands().iter().map(|c| c.id).collect::<Vec<_>>();
        let listed = ids(&s);
        assert!(!listed.contains(&"file.print") && !listed.contains(&"file.open") && !listed.contains(&"file.close"));
        assert!(listed.contains(&"home.bold") && listed.contains(&"insert.picture"));
        s.set_policy(&[("file.print".to_string(), true), ("home.bold".to_string(), false)]);
        let listed = ids(&s);
        assert!(listed.contains(&"file.print") && !listed.contains(&"home.bold"));
    }
}
