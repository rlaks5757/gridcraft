//! The `postMessage` bridge to a same-origin parent page (`?host=parent`), for pages that embed
//! GridCraft in an `<iframe>` (such as a Nextcloud app). Messages are plain objects with a `type`:
//!
//! - `gridcraft:ready` (to the parent): `{ version }`, the app is listening; send a file now.
//! - `gridcraft:open` (from the parent): `{ name, bytes, policy }`, bytes an `ArrayBuffer`
//!   or `Uint8Array`. It opens like a workbook picked with File › Open or dropped on the window.
//!   `policy` is `{ overrides: { "<command id>": true | false } }` with the commands that differ
//!   from their grade (see `commands.json`); in the PPM build `true` turns an off command on and
//!   `false` an on command off. A missing or malformed policy changes nothing; ids that are no
//!   command and values that are no boolean are ignored.
//! - `gridcraft:save` (to the parent): `{ name, bytes }` (a transferred `ArrayBuffer`) for every
//!   file GridCraft would otherwise download: Save and Save As (an .xlsx workbook) and Export
//!   as PDF. The parent stores it and reports failures itself.
//! - `gridcraft:dirty` (to the parent): `{ dirty }` whenever unsaved changes appear or go away.
//! - `gridcraft:failed` (to the parent): `{ error }` when the app couldn't start.
//! - `gridcraft:export` (from the parent, PPM build opened with `&jsontools=1` only): `{ id }`, a
//!   request for the active workbook as GridCraft JSON, as it is now (unsaved edits included).
//!   Any other frame ignores it. In such a frame `gridcraft:open` also replaces the open
//!   workbooks instead of adding a tab.
//! - `gridcraft:exported` (to the parent): `{ id, name, bytes }` (a transferred `ArrayBuffer` of
//!   JSON text) answering `gridcraft:export`; or `gridcraft:export-failed`: `{ id, error }`.
//!   Neither counts as a save: the workbook stays as dirty as it was.
//! - `gridcraft:request-bytes` (from the parent): `{ id, purpose }`, `purpose` one of `save`, `check`,
//!   `preview`: the workbook as it is now (unsaved edits included), in the format Save writes.
//! - `gridcraft:bytes` (to the parent): `{ id, purpose, name, bytes }` (a transferred `ArrayBuffer` of
//!   .xlsx) answering it. With `save` the workbook counts as saved, as after `gridcraft:save`;
//!   `check` and `preview` change nothing. `gridcraft:bytes-failed`: `{ id, purpose, reason }`, with
//!   `reason` `not-ready` (no workbook was opened yet), `edit-pending` or an error text.
//! - `gridcraft:add-fonts` (from the parent): `{ id, fonts: [{ family, bytes, format }] }`, `format`
//!   `ttf` or `otf`. Each font joins the font list and the drawing at once, replacing an installed
//!   or embedded font of the same name; a family sent twice is kept once. At most
//!   [`crate::gate::MAX_FONTS_PER_MESSAGE`] fonts count, each up to [`crate::gate::MAX_FONT_BYTES`], and
//!   [`crate::gate::MAX_FONTS_TOTAL_BYTES`] together; a family already added (any case) is kept once.
//! - `gridcraft:fonts-added` (to the parent): `{ id, installed, failed }`, the family names.
//!
//! Ctrl/Cmd+S inside the frame is GridCraft's Save (a `gridcraft:save`); the browser's own "save
//! page" never opens (see [`Bridge::listen`]).
//!
//! Only the parent window on the page's own origin is heard or answered. In host mode the page
//! starts with an empty workbook instead of a sample, and the parent guards leaving the page.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gridcraft_ui_egui::{Inbox, Services, SheetApp};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

/// Saving one of these hands the whole workbook to the parent, which stores it: the workbook
/// counts as saved under that name, as it would after Save on the desktop.
const WORKBOOK_EXTS: &[&str] = &["xlsx"];

/// The longest user name taken from `?author=`.
const MAX_AUTHOR_CHARS: usize = 200;

#[derive(Clone)]
pub struct Bridge {
    window: web_sys::Window,
    parent: web_sys::Window,
    origin: String,
    /// Names of the files handed to the parent since the last frame.
    saved: Rc<RefCell<Vec<String>>>,
    /// The unsaved-changes state last reported to the parent.
    reported: Rc<Cell<Option<bool>>>,
    /// The overrides of the latest `gridcraft:open`, until the next frame applies them.
    policy: Rc<RefCell<Option<Vec<(String, bool)>>>>,
    /// A `gridcraft:open` has arrived, so there is a workbook to answer `gridcraft:request-bytes` with.
    opened: Rc<Cell<bool>>,
    /// `gridcraft:request-bytes` requests (id, purpose) the next frame answers.
    byte_requests: Rc<RefCell<Vec<(String, &'static str)>>>,
    /// Ids of `gridcraft:export` requests the next frame answers.
    #[cfg(feature = "ppm-host")]
    exports: Rc<RefCell<Vec<String>>>,
    /// Opened with `&jsontools=1` (see [`crate::gate::json_tools`]): answers `gridcraft:export`.
    #[cfg(feature = "ppm-host")]
    json_tools: bool,
    /// A `gridcraft:open` arrived in a `json_tools` frame: the next frame closes the open workbooks first.
    #[cfg(feature = "ppm-host")]
    replace: Rc<Cell<bool>>,
}

impl Bridge {
    /// `None` when the page isn't framed (there is no parent to talk to).
    pub fn new() -> Option<Self> {
        let window = web_sys::window()?;
        let parent = window.parent().ok().flatten()?;
        if js_sys::Object::is(&parent, &window) {
            return None;
        }
        let origin = window.location().origin().ok()?;
        // A parent on another origin refuses to tell its origin: it is no PPM page.
        if !crate::gate::same_origin(parent.location().origin().ok().as_deref(), &origin) {
            return None;
        }
        Some(Self {
            window,
            parent,
            origin,
            saved: Rc::default(),
            reported: Rc::default(),
            policy: Rc::default(),
            opened: Rc::default(),
            byte_requests: Rc::default(),
            #[cfg(feature = "ppm-host")]
            exports: Rc::default(),
            #[cfg(feature = "ppm-host")]
            json_tools: false,
            #[cfg(feature = "ppm-host")]
            replace: Rc::default(),
        })
    }

    /// Turns on the JSON tools of the `/alternate-spreadjs` page (export on request, open replaces).
    #[cfg(feature = "ppm-host")]
    pub fn with_json_tools(mut self, on: bool) -> Self {
        self.json_tools = on;
        self
    }

    /// Before the frame handles the files that arrived: in a `json_tools` frame a new workbook replaces the open ones.
    #[cfg(feature = "ppm-host")]
    pub fn replace_documents(&self, app: &mut SheetApp) {
        if self.replace.take() {
            while !app.session.documents().is_empty() {
                app.session.close_document(0);
            }
        }
    }

    /// Send saved and exported files to the parent instead of downloading them, and hear files
    /// from it.
    pub fn connect(&self, services: &mut Services, inbox: Inbox, ctx: egui::Context) {
        self.listen(inbox, ctx);
        let bridge = self.clone();
        services.download = Some(Box::new(move |name: &str, bytes: &[u8]| {
            if let Err(e) = bridge.post_save(name, bytes) {
                log::error!("couldn't hand {name} to the page: {e}");
            }
        }));
    }

    /// `?author=`: the host's name for the user signs new notes and comments.
    pub fn set_author(app: &mut SheetApp, author: &str) {
        let author: String = author.trim().chars().take(MAX_AUTHOR_CHARS).collect();
        if !author.is_empty() {
            app.session.prefs.user_name = author;
        }
    }

    pub fn post_ready(&self) {
        self.send(&[("type", "gridcraft:ready".into()), ("version", env!("CARGO_PKG_VERSION").into())]);
    }

    pub fn post_failed(&self, error: &str) {
        self.send(&[("type", "gridcraft:failed".into()), ("error", error.into())]);
    }

    /// After each frame: a workbook handed to the parent this frame counts as saved under the
    /// name it was saved as, and the parent hears when unsaved changes appear or go away.
    pub fn sync(&self, app: &mut SheetApp) {
        // First, so a `save` request counts as saved in this very pass.
        self.answer_byte_requests(app);
        let saved = std::mem::take(&mut *self.saved.borrow_mut());
        if let Some(name) = saved.iter().rev().find(|name| is_workbook(name))
            && let Some(doc) = app.session.active_mut()
        {
            doc.saved = doc.wb.clone();
            if doc.path.is_none() {
                doc.title = name.clone();
            }
        }
        #[cfg(feature = "ppm-host")]
        self.answer_exports(app);
        let dirty = app.session.documents().iter().any(|d| d.is_dirty());
        if self.reported.get() != Some(dirty) && self.send(&[("type", "gridcraft:dirty".into()), ("dirty", dirty.into())]) {
            self.reported.set(Some(dirty));
        }
    }

    /// Answers the pending `gridcraft:request-bytes` requests with the active workbook as .xlsx.
    fn answer_byte_requests(&self, app: &mut SheetApp) {
        let requests = std::mem::take(&mut *self.byte_requests.borrow_mut());
        for (id, purpose) in requests {
            let sent = match self.workbook_bytes(app, purpose) {
                Ok((name, bytes)) => {
                    let buffer = js_sys::Uint8Array::from(bytes.as_slice()).buffer();
                    let sent = message(&[
                        ("type", "gridcraft:bytes".into()),
                        ("id", id.as_str().into()),
                        ("purpose", purpose.into()),
                        ("name", name.as_str().into()),
                        ("bytes", buffer.clone().into()),
                    ])
                    .is_some_and(|m| self.parent.post_message_with_transfer(&m, &self.origin, &js_sys::Array::of1(&buffer)).is_ok());
                    // Handed over for storing: it counts as saved under that name (see `sync`).
                    if sent && purpose == "save" {
                        self.saved.borrow_mut().push(name);
                    }
                    sent
                }
                Err(reason) => self.send(&[
                    ("type", "gridcraft:bytes-failed".into()),
                    ("id", id.as_str().into()),
                    ("purpose", purpose.into()),
                    ("reason", reason.as_str().into()),
                ]),
            };
            if !sent {
                log::error!("couldn't answer the bytes request {id}");
            }
        }
    }

    /// The active workbook as .xlsx and the name Save gives it. `save` first takes in a cell still
    /// being edited and the view (as Save does); `check` and `preview` leave the app untouched but
    /// still count that cell.
    fn workbook_bytes(&self, app: &mut SheetApp, purpose: &str) -> Result<(String, Vec<u8>), String> {
        if !self.opened.get() || app.session.active().is_none() {
            return Err("not-ready".into());
        }
        if purpose == "save" {
            if !app.commit_edit(0, 0, false, false) {
                return Err("edit-pending".into());
            }
            app.store_view_state();
        }
        // The same encoding and name as `file.saveBytes`, without its base64 round trip. A cell
        // still being edited is counted in (a copy takes it, the app doesn't move).
        app.xlsx_bytes_with_pending_edit().unwrap_or_else(|| Err("not-ready".into()))
    }

    /// Answers the pending `gridcraft:export` requests with the active workbook as JSON.
    #[cfg(feature = "ppm-host")]
    fn answer_exports(&self, app: &SheetApp) {
        let ids = std::mem::take(&mut *self.exports.borrow_mut());
        for id in ids {
            let json = app.session.active().ok_or_else(|| "no open workbook".to_string()).and_then(|doc| crate::gate::workbook_json(&doc.wb));
            let sent = match json {
                Ok(json) => {
                    let buffer = js_sys::Uint8Array::from(json.as_slice()).buffer();
                    message(&[
                        ("type", "gridcraft:exported".into()),
                        ("id", id.as_str().into()),
                        ("name", "workbook.json".into()),
                        ("bytes", buffer.clone().into()),
                    ])
                    .is_some_and(|m| self.parent.post_message_with_transfer(&m, &self.origin, &js_sys::Array::of1(&buffer)).is_ok())
                }
                Err(error) => self.send(&[("type", "gridcraft:export-failed".into()), ("id", id.as_str().into()), ("error", error.as_str().into())]),
            };
            if !sent {
                log::error!("couldn't answer the export request {id}");
            }
        }
    }

    fn post_save(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let buffer = js_sys::Uint8Array::from(bytes).buffer();
        let message = message(&[("type", "gridcraft:save".into()), ("name", name.as_str().into()), ("bytes", buffer.clone().into())])
            .ok_or_else(|| "couldn't build the message".to_string())?;
        self.parent
            .post_message_with_transfer(&message, &self.origin, &js_sys::Array::of1(&buffer))
            .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
        self.saved.borrow_mut().push(name);
        Ok(())
    }

    /// Before each pass: the policy that came with a workbook is set before the workbook opens.
    pub fn apply_policy(&self, app: &SheetApp) {
        if let Some(overrides) = self.policy.borrow_mut().take() {
            app.session.set_policy(&overrides);
        }
    }

    fn listen(&self, inbox: Inbox, ctx: egui::Context) {
        let parent = self.parent.clone();
        let policy = self.policy.clone();
        let opened = self.opened.clone();
        let byte_requests = self.byte_requests.clone();
        #[cfg(feature = "ppm-host")]
        let exports = self.exports.clone();
        #[cfg(feature = "ppm-host")]
        let json_tools = self.json_tools;
        #[cfg(feature = "ppm-host")]
        let replace = self.replace.clone();
        let origin = self.origin.clone();
        let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if event.origin() != origin || !event.source().is_some_and(|s| js_sys::Object::is(&s, &parent)) {
                return;
            }
            let data = event.data();
            #[cfg(feature = "ppm-host")]
            if string(&data, "type").as_deref() == Some("gridcraft:export") {
                if !json_tools {
                    return;
                }
                if let Some(id) = string(&data, "id").and_then(|id| crate::gate::export_id(&id)) {
                    exports.borrow_mut().push(id);
                    ctx.request_repaint();
                }
                return;
            }
            match string(&data, "type").as_deref() {
                Some("gridcraft:request-bytes") => {
                    if let Some(id) = string(&data, "id").and_then(|id| crate::gate::export_id(&id)) {
                        match string(&data, "purpose").as_deref().and_then(crate::gate::bytes_purpose) {
                            Some(purpose) => {
                                byte_requests.borrow_mut().push((id, purpose));
                                ctx.request_repaint();
                            }
                            None => {
                                let purpose = crate::gate::echo_purpose(&string(&data, "purpose").unwrap_or_default());
                                let fields = [
                                    ("type", "gridcraft:bytes-failed".into()),
                                    ("id", id.as_str().into()),
                                    ("purpose", purpose.as_str().into()),
                                    ("reason", "bad-purpose".into()),
                                ];
                                if !post_to(&parent, &origin, &fields) {
                                    log::error!("couldn't answer the bytes request {id}");
                                }
                            }
                        }
                    }
                    return;
                }
                Some("gridcraft:add-fonts") => {
                    if let Some(id) = string(&data, "id").and_then(|id| crate::gate::export_id(&id)) {
                        let (installed, failed) = add_fonts(&data);
                        let to_array = |names: Vec<String>| names.iter().map(|n| JsValue::from_str(n)).collect::<js_sys::Array>();
                        let fields = [
                            ("type", "gridcraft:fonts-added".into()),
                            ("id", id.as_str().into()),
                            ("installed", to_array(installed).into()),
                            ("failed", to_array(failed).into()),
                        ];
                        if !post_to(&parent, &origin, &fields) {
                            log::error!("couldn't answer the add-fonts request {id}");
                        }
                        // The app builds its font set again with them (see `SheetApp::logic`).
                        ctx.request_repaint();
                    }
                    return;
                }
                Some("gridcraft:open") => {}
                _ => return,
            }
            let name = string(&data, "name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Book.xlsx".to_string());
            let Some(bytes) = bytes(&data) else { return };
            *policy.borrow_mut() = Some(policy_overrides(&data));
            #[cfg(feature = "ppm-host")]
            if json_tools {
                replace.set(true);
            }
            inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes));
            opened.set(true);
            ctx.request_repaint();
        });
        // Ctrl/Cmd+S is GridCraft's Save, never the browser's "save page". eframe already stops
        // it while the canvas has the keyboard; this covers the moments it doesn't (such as just
        // after the frame loads). It only cancels the default: the key still reaches GridCraft.
        let on_key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|event: web_sys::KeyboardEvent| {
            if (event.ctrl_key() || event.meta_key()) && (event.code() == "KeyS" || event.key().eq_ignore_ascii_case("s")) {
                event.prevent_default();
            }
        });
        if self.window.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref()).is_ok() {
            on_key.forget();
        }
        if self.window.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref()).is_ok() {
            // The listener lives as long as the page.
            on_message.forget();
        }
    }

    /// Post a message to the parent; false if it couldn't be sent.
    fn send(&self, fields: &[(&str, JsValue)]) -> bool {
        post_to(&self.parent, &self.origin, fields)
    }
}

/// Post a message to `parent`; false if it couldn't be sent.
fn post_to(parent: &web_sys::Window, origin: &str, fields: &[(&str, JsValue)]) -> bool {
    message(fields).is_some_and(|m| parent.post_message(&m, origin).is_ok())
}

/// Installs the fonts of a `gridcraft:add-fonts` message; returns the families installed (a family
/// already there counts) and the ones that failed (no name, wrong format, too big, not a font).
fn add_fonts(data: &JsValue) -> (Vec<String>, Vec<String>) {
    let (mut installed, mut failed) = (Vec::new(), Vec::new());
    let Some(list) = js_sys::Reflect::get(data, &JsValue::from_str("fonts")).ok().filter(js_sys::Array::is_array) else {
        return (installed, failed);
    };
    // Bytes taken so far in this message: the whole message is capped, not only each file.
    let mut taken = 0usize;
    for (i, item) in js_sys::Array::from(&list).iter().enumerate() {
        let family = string(&item, "family").unwrap_or_default();
        if i >= crate::gate::MAX_FONTS_PER_MESSAGE || !string(&item, "format").is_some_and(|f| crate::gate::font_format_ok(&f)) {
            failed.push(family);
        } else if gridcraft_ui_egui::fonts::has_uploaded(&family) {
            // Already there: nothing to copy.
            installed.push(family);
        } else {
            // The size is read off the JS buffer, so a too-big font is never copied into the app.
            let len = byte_len(&item).filter(|len| *len <= crate::gate::MAX_FONT_BYTES && taken + len <= crate::gate::MAX_FONTS_TOTAL_BYTES);
            match len.and_then(|len| bytes(&item).map(|b| (len, b))) {
                Some((len, b)) => match gridcraft_ui_egui::fonts::add_uploaded(&family, b) {
                    Ok(()) => {
                        taken += len;
                        installed.push(family);
                    }
                    Err(e) => {
                        log::warn!("font {family:?} refused: {e}");
                        failed.push(family);
                    }
                },
                None => failed.push(family),
            }
        }
    }
    (installed, failed)
}

/// The length of `bytes` (an `ArrayBuffer` or `Uint8Array`) without copying it.
fn byte_len(data: &JsValue) -> Option<usize> {
    let bytes = js_sys::Reflect::get(data, &JsValue::from_str("bytes")).ok()?;
    if let Some(array) = bytes.dyn_ref::<js_sys::Uint8Array>() {
        Some(array.length() as usize)
    } else {
        bytes.dyn_ref::<js_sys::ArrayBuffer>().map(|b| b.byte_length() as usize)
    }
}

fn is_workbook(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| WORKBOOK_EXTS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

fn message(fields: &[(&str, JsValue)]) -> Option<js_sys::Object> {
    let m = js_sys::Object::new();
    for (key, value) in fields {
        js_sys::Reflect::set(&m, &JsValue::from_str(key), value).ok()?;
    }
    Some(m)
}

/// `policy.overrides` as `(command id, on)` pairs (see [`crate::gate::policy_overrides`]).
fn policy_overrides(data: &JsValue) -> Vec<(String, bool)> {
    let text = js_sys::Reflect::get(data, &JsValue::from_str("policy"))
        .ok()
        .and_then(|policy| js_sys::JSON::stringify(&policy).ok())
        .and_then(|text| text.as_string());
    text.map(|text| crate::gate::policy_overrides(&text)).unwrap_or_default()
}

fn string(data: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(data, &JsValue::from_str(key)).ok()?.as_string()
}

/// `bytes` as an `ArrayBuffer` or `Uint8Array`.
fn bytes(data: &JsValue) -> Option<Vec<u8>> {
    let bytes = js_sys::Reflect::get(data, &JsValue::from_str("bytes")).ok()?;
    if let Some(array) = bytes.dyn_ref::<js_sys::Uint8Array>() {
        Some(array.to_vec())
    } else if bytes.is_instance_of::<js_sys::ArrayBuffer>() {
        Some(js_sys::Uint8Array::new(&bytes).to_vec())
    } else {
        None
    }
}
