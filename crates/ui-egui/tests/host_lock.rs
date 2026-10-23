//! PPM host lock: a locked app keeps the editor and Save, and loses every way to reach local
//! files, printing, exports, scripts and links. The engine side is tested in `hostlock.rs`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{Inbox, Services, SheetApp};
use serde_json::json;

type Downloads = Arc<Mutex<Vec<String>>>;

/// Services that count every use of a platform feature the lock must remove.
fn counting(hits: &Arc<AtomicUsize>, downloads: &Downloads, inbox: Inbox) -> Services {
    let hit = || {
        let hits = hits.clone();
        move || {
            hits.fetch_add(1, Ordering::SeqCst);
        }
    };
    let (open, pick_picture_async, url, print) = (hit(), hit(), hit(), hit());
    let (pick_open, pick_save, pick_picture) = (hit(), hit(), hit());
    let downloads = downloads.clone();
    Services {
        open_async: Some(Box::new(open)),
        pick_picture_async: Some(Box::new(move |_| pick_picture_async())),
        open_url: Some(Box::new(move |_| url())),
        print: Some(Box::new(move |_| print())),
        pick_open: Some(Box::new(move || {
            pick_open();
            None
        })),
        pick_save: Some(Box::new(move |_| {
            pick_save();
            None
        })),
        pick_picture: Some(Box::new(move || {
            pick_picture();
            None
        })),
        download: Some(Box::new(move |name, _| downloads.lock().unwrap().push(name.to_string()))),
        inbox: Some(inbox),
        ..Default::default()
    }
}

fn harness(locked: bool, services: Services) -> egui_kittest::Harness<'static, SheetApp> {
    harness_with(locked, &[], services)
}

/// A harness whose parent turned `overrides` on (`true`) or off (`false`).
fn harness_with(locked: bool, overrides: &[(&str, bool)], services: Services) -> egui_kittest::Harness<'static, SheetApp> {
    let mut session = Session::new();
    if locked {
        session.lock_host();
        session.set_policy(&overrides.iter().map(|(id, on)| (id.to_string(), *on)).collect::<Vec<_>>());
    }
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        SheetApp::new(session, services),
    );
    h.run_steps(4);
    h
}

fn locked_harness() -> (egui_kittest::Harness<'static, SheetApp>, Arc<AtomicUsize>, Downloads, Inbox) {
    let hits = Arc::new(AtomicUsize::new(0));
    let downloads = Downloads::default();
    let inbox = Inbox::default();
    let h = harness(true, counting(&hits, &downloads, inbox.clone()));
    (h, hits, downloads, inbox)
}

#[test]
fn blocked_commands_do_nothing_quietly() {
    let (mut h, hits, downloads, _) = locked_harness();
    for id in [
        "file.new",
        "file.open",
        "file.close",
        "file.saveAs",
        "file.print",
        "file.exportPdf",
        "file.exportCsv",
        "file.exportHtml",
        "link.open",
        "view.macros",
        "automate.runScript",
        "ui.printSettings",
        "ui.printSubmit",
    ] {
        h.state_mut().run_or_alert(id, json!({}));
        h.run_steps(2);
        assert!(h.state().message.is_none() && h.state().toast.is_none(), "{id} must refuse without a message");
    }
    assert_eq!(h.state().session.documents().len(), 1, "no workbook was created or closed");
    assert_eq!(hits.load(Ordering::SeqCst), 0, "no picker, print or URL service ran");
    assert!(downloads.lock().unwrap().is_empty(), "nothing was handed to the host");
}

#[test]
fn shortcuts_for_blocked_commands_do_nothing() {
    let (mut h, hits, downloads, _) = locked_harness();
    for (key, modifiers) in [
        (egui::Key::O, egui::Modifiers::COMMAND),
        (egui::Key::P, egui::Modifiers::COMMAND),
        (egui::Key::N, egui::Modifiers::COMMAND),
        (egui::Key::S, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT),
        (egui::Key::F12, egui::Modifiers::NONE),
    ] {
        h.key_press_modifiers(modifiers, key);
        h.run_steps(3);
    }
    assert_eq!(h.state().session.documents().len(), 1);
    assert!(h.state().dialog.is_none() && h.state().backstage.is_none() && h.state().message.is_none());
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert!(downloads.lock().unwrap().is_empty());
}

#[test]
fn blocked_dialogs_do_not_open() {
    let (mut h, hits, _, _) = locked_harness();
    for name in ["open", "start", "saveCopy", "macros", "openLink", "agents"] {
        h.state_mut().open_dialog(name, json!({"url": "https://example.com"}));
        h.run_steps(2);
        assert!(h.state().dialog.is_none(), "{name} opened");
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0, "the open dialog did not start a pick");
}

#[test]
fn save_reaches_the_host_as_xlsx_and_nothing_else_does() {
    let (mut h, _, downloads, _) = locked_harness();
    h.state_mut().run_or_alert("file.save", json!({}));
    h.run_steps(3);
    {
        let saved = downloads.lock().unwrap();
        assert_eq!(saved.len(), 1, "Save hands one file to the host");
        assert!(saved[0].ends_with(".xlsx"), "{}", saved[0]);
    }
    let download = h.state().services.download.as_ref().unwrap();
    for name in ["Book1.pdf", "Book1.csv", "Book1.html", "Book1", "Book1.xlsx.exe"] {
        download(name, b"x");
    }
    assert_eq!(downloads.lock().unwrap().len(), 1, "only a workbook may reach the host");
    download("Book1.XLSX", b"x");
    assert_eq!(downloads.lock().unwrap().len(), 2);
}

#[test]
fn the_file_tab_shows_only_save_and_options() {
    let (mut h, _, downloads, _) = locked_harness();
    h.get_by_label("File").click();
    h.run_steps(3);
    assert_eq!(h.state().backstage.as_ref().map(|b| b.page.id()), Some("options"));
    for label in ["Home", "New", "Open", "Info", "Save As", "Print", "Export", "Close"] {
        assert!(h.query_by_role_and_label(egui::accesskit::Role::Button, label).is_none(), "{label} is in the File tab");
    }
    for page in ["home", "new", "open", "info", "saveAs", "print", "export"] {
        h.state_mut().run("ui.backstage", json!({"page": page})).unwrap();
        h.run_steps(2);
        assert_eq!(h.state().backstage.as_ref().map(|b| b.page.id()), Some("options"), "{page} opened");
    }
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Save").click();
    h.run_steps(3);
    assert_eq!(downloads.lock().unwrap().len(), 1, "Save in the File tab goes to the host");
    assert!(h.state().backstage.is_none());
}

#[test]
fn the_ribbon_and_title_bar_lose_the_blocked_entries() {
    let (mut h, _, _, _) = locked_harness();
    assert!(h.query_by_label("AutoSave").is_none());
    h.state_mut().ui.ribbon_tab = "Insert".into();
    h.run_steps(3);
    assert!(h.query_by_label("Pictures").is_some(), "a picture goes in the workbook");
    h.state_mut().ui.ribbon_tab = "Data".into();
    h.run_steps(3);
    assert!(h.query_by_label("Get Data\n(Text/CSV)").is_none(), "Get Data reads a local file");

    let hits = Arc::new(AtomicUsize::new(0));
    let mut open = harness(false, counting(&hits, &Downloads::default(), Inbox::default()));
    assert!(open.query_by_label("AutoSave").is_some(), "the upstream app keeps its AutoSave switch");
    open.state_mut().ui.ribbon_tab = "Insert".into();
    open.run_steps(3);
    assert!(open.query_by_label("Pictures").is_some());
}

#[test]
fn autosave_stays_off() {
    let (mut h, _, downloads, _) = locked_harness();
    assert!(!h.state().grid.autosave());
    h.state_mut().grid.autosave = true;
    h.state_mut().run("cell.set", json!({"ref": "A1", "value": "1"})).unwrap();
    h.run_steps(4);
    assert!(downloads.lock().unwrap().is_empty());
}

#[test]
fn the_host_can_still_open_a_workbook_through_the_inbox() {
    let mut source = Session::new();
    source.new_workbook();
    source.run("cell.set", json!({"ref": "A1", "value": "from host"})).unwrap();
    let b64 = source.run("file.saveBytes", json!({"format": "xlsx"})).unwrap()["base64"].as_str().unwrap().to_string();
    let bytes = gridcraft_engine::io::base64_decode(&b64).unwrap();

    let (mut h, _, _, inbox) = locked_harness();
    inbox.lock().unwrap().push(("Plan.xlsx".into(), bytes));
    h.run_steps(3);
    assert!(h.state().message.is_none());
    let doc = h.state().session.active().unwrap();
    assert!(doc.display_title().contains("Plan"), "the workbook the host sent opened: {}", doc.display_title());
    assert_eq!(doc.wb.active().unwrap().value(gridcraft_engine::core::CellRef::new(0, 0)), gridcraft_engine::core::Value::Text("from host".into()));
}

#[test]
fn an_unlocked_app_keeps_every_service() {
    let hits = Arc::new(AtomicUsize::new(0));
    let h = harness(false, counting(&hits, &Downloads::default(), Inbox::default()));
    let s = &h.state().services;
    assert!(s.open_async.is_some() && s.pick_open.is_some() && s.pick_save.is_some() && s.pick_picture.is_some());
    assert!(s.pick_picture_async.is_some() && s.open_url.is_some() && s.print.is_some());
    assert!(!h.state().host_locked());
}

#[test]
fn the_locked_app_keeps_the_pickers_for_things_that_go_in_the_workbook() {
    let (h, ..) = locked_harness();
    let s = &h.state().services;
    assert!(s.pick_picture.is_some() && s.pick_picture_async.is_some());
    assert!(s.open_async.is_none() && s.pick_open.is_none() && s.pick_save.is_none());
}

#[test]
fn a_picture_can_still_be_inserted() {
    let (mut h, ..) = locked_harness();
    let png = {
        let mut out = Vec::new();
        image::DynamicImage::new_rgba8(2, 2).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    };
    let r = h.state_mut().run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&png), "at": "B2"}));
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn download_and_print_and_links_follow_the_policy() {
    for (overrides, name, passes) in [
        (&[][..], "a.xlsx", true),
        (&[][..], "a.pdf", false),
        (&[("file.print", true)][..], "a.pdf", true),
        (&[("file.print", true)][..], "a.csv", false),
        (&[("file.exportCsv", true)][..], "a.csv", true),
        (&[("file.exportCsv", true)][..], "a.html", false),
        (&[("file.exportHtml", true)][..], "a.html", true),
        (&[("file.saveAs", true)][..], "a.json", true),
        (&[][..], "a.json", false),
    ] {
        let downloads = Downloads::default();
        let h = harness_with(true, overrides, counting(&Arc::default(), &downloads, Inbox::default()));
        (h.state().services.download.as_ref().unwrap())(name, b"x");
        assert_eq!(!downloads.lock().unwrap().is_empty(), passes, "{name} with {overrides:?}");
    }
    for (overrides, runs) in [(&[][..], 0), (&[("link.open", true), ("file.print", true)][..], 1)] {
        let hits = Arc::new(AtomicUsize::new(0));
        let h = harness_with(true, overrides, counting(&hits, &Downloads::default(), Inbox::default()));
        (h.state().services.open_url.as_ref().unwrap())("https://example.com");
        (h.state().services.print.as_ref().unwrap())(b"x");
        assert_eq!(hits.load(Ordering::SeqCst), runs * 2, "{overrides:?}");
    }
}

#[test]
fn overrides_turn_commands_and_pages_on_and_off() {
    let (mut h, ..) = {
        let hits = Arc::new(AtomicUsize::new(0));
        (harness_with(true, &[("file.print", true), ("file.exportPdf", true), ("home.bold", false), ("review.protectWorkbook", true)], counting(&hits, &Downloads::default(), Inbox::default())),)
    };
    assert!(h.state().page_allowed(gridcraft_ui_egui::backstage::Page::Print));
    assert!(h.state().page_allowed(gridcraft_ui_egui::backstage::Page::Export));
    assert!(!h.state().page_allowed(gridcraft_ui_egui::backstage::Page::SaveAs));
    assert!(!h.state().page_allowed(gridcraft_ui_egui::backstage::Page::Open));
    assert!(!h.state().page_allowed(gridcraft_ui_egui::backstage::Page::Info), "Info is hidden in PPM");
    assert!(h.state().page_allowed(gridcraft_ui_egui::backstage::Page::Options));
    // Protect Workbook stays locked whatever the policy says.
    assert!(!h.state().allowed("review.protectWorkbook"));
    assert!(h.state_mut().run("review.protectWorkbook", json!({"on": true})).is_err());
    assert!(h.state_mut().run("review.protectWorkbook", json!({"on": false})).is_err());
    assert!(!h.state().allowed("home.bold") && h.state().allowed("home.italic"));
    assert!(h.state_mut().run("home.bold", json!({})).is_err());
    assert!(h.state_mut().run("file.close", json!({})).is_err(), "locked stays locked");
    assert!(h.state().allowed("ui.printSettings") && h.state().allowed("ui.printSubmit"), "the print dialog follows file.print");
}

/// Every command the app handles itself (`run_ui_command`) is classified for the host lock.
#[test]
fn ui_commands_are_all_classified() {
    let src = include_str!("../src/lib.rs");
    let body = src.split("fn run_ui_command").nth(1).and_then(|b| b.split("_ => return None").next()).expect("run_ui_command");
    let mut found = Vec::new();
    for line in body.lines() {
        // An arm sits exactly at the match's indentation: `"id" | "id2" => …`.
        if let Some(arms) = line.strip_prefix("            \"").and_then(|l| l.split_once(" =>")) {
            found.extend(arms.0.split(" | ").map(|id| id.trim_matches('"').to_string()));
        }
    }
    assert!(found.len() >= 15, "found only {found:?}");
    for id in &found {
        let lists = [gridcraft_ui_egui::UI_COMMANDS_ALLOWED, gridcraft_ui_egui::UI_COMMANDS_OFF, gridcraft_ui_egui::UI_COMMANDS_PLUMBING];
        let n = lists.iter().filter(|l| l.contains(&id.as_str())).count();
        assert_eq!(n, 1, "app command `{id}` must be in exactly one of UI_COMMANDS_ALLOWED / UI_COMMANDS_OFF / UI_COMMANDS_PLUMBING");
    }
    for id in gridcraft_ui_egui::UI_COMMANDS_ALLOWED.iter().chain(gridcraft_ui_egui::UI_COMMANDS_OFF).chain(gridcraft_ui_egui::UI_COMMANDS_PLUMBING) {
        assert!(found.iter().any(|f| f == id), "`{id}` is classified but run_ui_command has no such arm");
    }
}

#[test]
fn a_copy_dialog_opens_for_the_export_that_is_on() {
    for (overrides, format, opens) in [
        (&[("file.exportCsv", true)][..], "csv", true),
        (&[("file.exportCsv", true)][..], "html", false),
        (&[("file.exportCsv", true)][..], "xlsx", false),
        (&[("file.exportHtml", true)][..], "html", true),
        (&[("file.exportPdf", true)][..], "pdf", true),
        (&[("file.saveAs", true)][..], "xlsx", true),
        (&[("file.saveAs", true)][..], "csv", false),
        (&[][..], "csv", false),
    ] {
        let mut h = harness_with(true, overrides, Services::default());
        h.state_mut().open_dialog("saveCopy", json!({"format": format}));
        h.run_steps(2);
        assert_eq!(h.state().dialog.is_some(), opens, "{format} with {overrides:?}");
    }
}

#[test]
fn an_app_command_that_is_off_is_refused_without_a_word() {
    let mut h = harness_with(true, &[("view.animations", false)], Services::default());
    let e = h.state_mut().run("view.animations", json!({})).unwrap_err();
    assert!(e.contains(gridcraft_engine::hostlock::BLOCKED_TEXT), "{e}");
    h.state_mut().run_or_alert("view.animations", json!({}));
    h.run_steps(2);
    assert!(h.state().message.is_none(), "no message box for a command that is switched off");
    assert!(h.state_mut().run("view.collapseRibbon", json!({})).is_ok());
    assert!(h.state_mut().run("ui.ribbonTab", json!({"tab": "Insert"})).is_ok(), "the plumbing is no switch");
}

#[test]
fn get_data_fills_a_sheet_and_never_swaps_the_workbook() {
    let services = Services {
        pick_data_async: Some(Box::new(|inbox: Inbox| inbox.lock().unwrap().push(("data.csv".into(), b"a,b\n1,2\n".to_vec())))),
        ..Default::default()
    };
    let mut h = harness_with(true, &[], services);
    let sheets = |h: &egui_kittest::Harness<'static, SheetApp>| h.state().session.active().map_or(0, |d| d.wb.sheets.len());
    let before = sheets(&h);
    // Get Data without a file asks the app to pick one, not to open a workbook.
    h.state_mut().run_or_alert("data.getData", json!({}));
    h.run_steps(2);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("importData"));
    h.get_by_label("Browse…").click();
    h.run_steps(4);
    assert_eq!(h.state().session.documents().len(), 1, "the file did not become a workbook");
    assert_eq!(sheets(&h), before + 1, "the data is a sheet of the open workbook");
    assert!(h.state().dialog.is_none() && h.state().message.is_none());
    // A path would read a local file, and the Open dialog stays shut.
    assert!(h.state_mut().run("data.getData", json!({"path": "/etc/hosts"})).is_err());
    h.state_mut().open_dialog("open", json!({}));
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
    // Off: no dialog.
    let mut h = harness_with(true, &[("data.getData", false)], Services::default());
    h.state_mut().open_dialog("importData", json!({}));
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
}

#[test]
fn the_save_as_and_export_pages_offer_only_the_formats_that_are_on() {
    let mut h = harness_with(true, &[("file.exportCsv", true)], counting(&Arc::default(), &Downloads::default(), Inbox::default()));
    h.get_by_label("File").click();
    h.run_steps(3);
    h.state_mut().run("ui.backstage", json!({"page": "export"})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label_contains("CSV (Comma delimited)").is_some());
    assert!(h.query_by_label_contains("Web Page").is_none() && h.query_by_label_contains("Create PDF").is_none());
}

#[test]
fn share_menu_exports_reach_the_host() {
    let downloads = Downloads::default();
    let mut h = harness_with(true, &[("file.exportCsv", true), ("file.exportHtml", true), ("view.collapseRibbon", false)], counting(&Arc::default(), &downloads, Inbox::default()));
    // The Share menu's path: no save picker in the locked web app, so the file goes to the host.
    h.state_mut().services.pick_save = None;
    assert!(gridcraft_ui_egui::backstage::export_as(h.state_mut(), "csv"));
    assert!(gridcraft_ui_egui::backstage::export_as(h.state_mut(), "html"));
    assert_eq!(downloads.lock().unwrap().len(), 2, "{:?}", downloads.lock().unwrap());
}

#[test]
fn the_data_commands_are_one_switch_and_plumbing_ignores_the_policy() {
    for off in ["data.getData", "data.fromTextCsv"] {
        let mut h = harness_with(true, &[(off, false), ("ui.ribbonTab", false), ("ui.dialog", false)], Services::default());
        for id in ["data.getData", "data.fromTextCsv"] {
            assert!(h.state_mut().run(id, json!({"text": "a\n1\n"})).is_err(), "{id} with {off} off");
        }
        assert!(h.state().allowed("ui.ribbonTab") && h.state().allowed("ui.dialog"));
        // The button and the dialog follow: neither survives when either command is off.
        assert!(!h.state().allowed("data.getData") && h.state().switched_off("data.getData"), "{off}");
        h.state_mut().open_dialog("importData", json!({}));
        h.run_steps(2);
        assert!(h.state().dialog.is_none(), "{off}");
        assert!(h.state_mut().run("ui.ribbonTab", json!({"tab": "Insert"})).is_ok());
    }
}

#[test]
fn save_as_with_the_command_off_sends_what_save_sends() {
    // Save has no file to write to in the locked web app, so it asks for the same dialog Ctrl+Shift+S
    // opens: the workbook as xlsx under its own name. Both must hand the host the same thing.
    let downloads = Downloads::default();
    let mut h = harness(true, Services { download: Some(Box::new({
        let downloads = downloads.clone();
        move |name, bytes| downloads.lock().unwrap().push(format!("{name}:{}", bytes.len()))
    })), ..Default::default() });
    h.state_mut().run_or_alert("file.save", json!({}));
    h.run_steps(2);
    h.state_mut().open_dialog("saveAs", json!({}));
    h.run_steps(2);
    let got = downloads.lock().unwrap().clone();
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0], got[1], "Save As (off) and Save send the same file under the same name");
    assert!(got[0].contains(".xlsx:"), "{got:?}");
}

/// The PPM build is always Korean: the language commands refuse, so a stored `en` or a menu
/// choice cannot change it. The unlocked app still switches.
#[test]
fn the_interface_language_is_fixed_when_locked() {
    use gridcraft_ui_egui::i18n::Language;
    let (mut h, ..) = locked_harness();
    h.state_mut().ui.language = Language::Ko;
    for (id, params) in [("app.language.set", json!({"language": "en"})), ("app.language.english", json!({})), ("app.language.japanese", json!({}))] {
        assert!(h.state_mut().run(id, params).is_err(), "{id}");
        assert_eq!(h.state().ui.language, Language::Ko, "{id}");
    }
    let mut open = harness(false, Services::default());
    open.state_mut().run("app.language.set", json!({"language": "de"})).unwrap();
    assert_eq!(open.state().ui.language, Language::De);
}

/// A filled-in text (a link, a sheet name) is shown as it is: a `|` in it is not a `context|label`
/// separator, in English or Korean.
#[test]
fn a_pipe_in_the_link_to_confirm_is_shown_whole() {
    use gridcraft_ui_egui::i18n::Language;
    let url = "https://evil.example/?a|https://trusted.example";
    for language in [Language::En, Language::Ko] {
        let mut h = harness(false, Services::default());
        h.state_mut().ui.language = language;
        h.state_mut().open_dialog("openLink", json!({"url": url}));
        h.run_steps(4);
        assert!(h.query_by_label_contains(url).is_some(), "{language:?}: the whole address is shown");
    }
}
