//! PPM host lock: what an embedded GridCraft may do when a same-origin parent page owns the
//! files. In a locked session (every session of the `ppm-host` build, see [`HOST_BUILD`]) a
//! command runs only if it is classified here; a command nobody classified is refused, and the
//! drift test below fails until it is classified.
//!
//! Blocked: opening, creating, closing and exporting files, printing, scripts, local pictures and
//! opening links. The workbook itself reaches the parent through `file.save` (xlsx only).

use serde_json::Value as Json;

/// True in builds made with the `ppm-host` feature (`ppm/build-web.sh`): every session is locked
/// and nothing at run time can unlock it.
pub const HOST_BUILD: bool = cfg!(feature = "ppm-host");

/// Text every host-lock refusal carries, so the UI can drop it without a message.
pub const BLOCKED_TEXT: &str = "blocked by the PPM host lock";

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
/// Commands a locked session refuses.
pub const BLOCKED: &[&str] = &[
    "automate.allScripts",
    "automate.deleteScript",
    "automate.newScript",
    "automate.recordActions",
    "automate.runScript",
    "data.fromTextCsv",
    "data.getData",
    "file.close",
    "file.exportCsv",
    "file.exportHtml",
    "file.exportPdf",
    "file.new",
    "file.print",
    "file.saveAs",
    "insert.picture",
    "link.open",
    "pageLayout.background",
    "view.macros",
];
/// Commands a locked session runs only with certain params (see [`permits`]).
pub const CONDITIONAL: &[&str] = &[
    "file.open",
    "file.save",
    "file.saveBytes",
];

/// Whether a locked session may run `id` with `params`. Unclassified ids are refused.
pub fn permits(id: &str, params: &Json) -> bool {
    let has = |key: &str| params.get(key).is_some_and(|v| !v.is_null());
    match id {
        // A workbook the parent sent arrives as bytes; no path, no picker.
        "file.open" => has("base64") && !has("path"),
        // Save hands the xlsx to the parent; a path would pick a file (and its extension a format).
        "file.save" => !has("path"),
        "file.saveBytes" => params.get("format").and_then(Json::as_str).is_none_or(|f| f.eq_ignore_ascii_case("xlsx")),
        _ => ALLOWED.contains(&id),
    }
}

/// Whether a locked session lists `id` (command search, menus built from the command table).
pub fn listed(id: &str) -> bool {
    ALLOWED.contains(&id)
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
            let n = [ALLOWED, BLOCKED, CONDITIONAL].iter().filter(|l| l.contains(&spec.id)).count();
            assert_eq!(n, 1, "command `{}` must be in exactly one of ALLOWED, BLOCKED, CONDITIONAL in hostlock.rs", spec.id);
        }
    }

    #[test]
    fn lists_hold_no_stale_ids_and_are_sorted() {
        for list in [ALLOWED, BLOCKED, CONDITIONAL] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
            for id in list {
                assert!(command_specs().iter().any(|c| c.id == *id), "`{id}` is classified but no such command exists");
            }
        }
    }

    #[test]
    fn locked_session_refuses_every_blocked_command() {
        let mut s = locked();
        for id in BLOCKED {
            let e = s.execute(id, json!({})).unwrap_err().to_string();
            assert!(e.contains(BLOCKED_TEXT), "`{id}` ran in a locked session: {e}");
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
        assert!(!permits("file.open", &json!({"path": "a.xlsx", "base64": "AA=="})));
        assert!(permits("file.open", &json!({"name": "a.xlsx", "base64": "AA=="})));
    }

    #[test]
    fn save_is_xlsx_bytes_only() {
        let mut s = locked();
        assert!(s.execute("file.saveBytes", json!({"format": "xlsx"})).is_ok());
        for format in ["csv", "html", "json", "tsv"] {
            let e = s.execute("file.saveBytes", json!({"format": format})).unwrap_err().to_string();
            assert!(e.contains(BLOCKED_TEXT), "{format}");
        }
        assert!(!permits("file.save", &json!({"path": "x.pdf"})));
        assert!(permits("file.save", &json!({})));
    }

    #[test]
    fn locked_commands_are_not_listed() {
        let s = locked();
        let ids: Vec<&str> = s.commands().iter().map(|c| c.id).collect();
        assert!(!ids.contains(&"file.print") && !ids.contains(&"file.open"));
        assert!(ids.contains(&"home.bold"));
    }
}
