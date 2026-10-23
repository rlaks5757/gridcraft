//! The sample templates of the PPM `/alternate-spreadjs` test page (JSON workbooks with `{{tag}}` cells).
//!
//! `PPM_SAMPLES_OUT=<dir> cargo test -p gridcraft-engine --test spreadjs_samples` writes them as `<dir>/<name>.json`.
//! `PPM_SAMPLES_DIR=<dir>` makes `every_json_in_the_directory_opens` open each `*.json` there the way the app does,
//! which also checks workbooks the page filled in.

use std::path::Path;

use gridcraft_core::{CellRef, Value};
use gridcraft_engine::io;
use gridcraft_model::{Cell, Color, Fill, LineInfo, Sheet, StyleId, Workbook};

struct Styles {
    title: StyleId,
    label: StyleId,
    head: StyleId,
    money: StyleId,
}

fn styles(wb: &mut Workbook) -> Styles {
    let normal = StyleId::DEFAULT;
    let title = wb.styles.derive(normal, |s| {
        s.font.bold = true;
        s.font.size = 16.0;
    });
    let label = wb.styles.derive(normal, |s| s.font.bold = true);
    let head = wb.styles.derive(normal, |s| {
        s.font.bold = true;
        s.fill = Fill::solid(Color::Rgb(0xDDEBF7));
    });
    let money = wb.styles.derive(normal, |s| s.num_fmt = gridcraft_model::NumFmt::new("#,##0"));
    Styles { title, label, head, money }
}

fn put(sh: &mut Sheet, at: &str, text: &str, style: StyleId) {
    let Some(cell) = CellRef::parse(at) else { panic!("bad cell {at}") };
    sh.cells.set(cell, Cell { value: Value::Text(text.into()), formula: None, style });
}

fn widths(sh: &mut Sheet, widths: &[f32]) {
    for (col, w) in widths.iter().enumerate() {
        sh.cols.insert(col as u32, LineInfo { size: Some(*w), ..LineInfo::default() });
    }
}

fn workbook(name: &str, build: impl FnOnce(&mut Sheet, &Styles)) -> Workbook {
    let mut wb = Workbook::new();
    let st = styles(&mut wb);
    let mut sh = Sheet::new(name);
    build(&mut sh, &st);
    wb.sheets = vec![std::sync::Arc::new(sh)];
    wb
}

/// (file name, workbook). Tags: `{{key}}`; a row from `{{loop name}}` to `{{endloop}}` repeats once per item.
fn samples() -> Vec<(&'static str, Workbook)> {
    let quote = workbook("견적서", |sh, st| {
        widths(sh, &[180.0, 70.0, 100.0, 110.0]);
        put(sh, "A1", "견적서", st.title);
        put(sh, "A3", "고객사", st.label);
        put(sh, "B3", "{{customer}}", StyleId::DEFAULT);
        put(sh, "A4", "발행일", st.label);
        put(sh, "B4", "{{issuedAt}}", StyleId::DEFAULT);
        for (c, h) in ["품목", "수량", "단가", "금액"].iter().enumerate() {
            put(sh, &format!("{}6", (b'A' + c as u8) as char), h, st.head);
        }
        put(sh, "A7", "{{loop items}}{{name}}", StyleId::DEFAULT);
        put(sh, "B7", "{{qty}}", st.money);
        put(sh, "C7", "{{price}}", st.money);
        put(sh, "D7", "{{amount}}{{endloop}}", st.money);
        put(sh, "A9", "합계", st.label);
        put(sh, "D9", "{{total}}", st.money);
    });
    let statement = workbook("거래명세서", |sh, st| {
        widths(sh, &[160.0, 70.0, 100.0, 110.0, 160.0]);
        put(sh, "A1", "거래명세서 - {{customer}} 귀하", st.title);
        put(sh, "A3", "거래일", st.label);
        put(sh, "B3", "{{date}}", StyleId::DEFAULT);
        for (c, h) in ["품명", "수량", "단가", "공급가액", "비고"].iter().enumerate() {
            put(sh, &format!("{}5", (b'A' + c as u8) as char), h, st.head);
        }
        put(sh, "A6", "{{loop lines}}{{item}}", StyleId::DEFAULT);
        put(sh, "B6", "{{qty}}", st.money);
        put(sh, "C6", "{{unitPrice}}", st.money);
        put(sh, "D6", "{{supply}}", st.money);
        put(sh, "E6", "{{memo}}{{endloop}}", StyleId::DEFAULT);
        put(sh, "A8", "공급가액 합계", st.label);
        put(sh, "D8", "{{supplyTotal}}", st.money);
        put(sh, "A9", "부가세", st.label);
        put(sh, "D9", "{{vat}}", st.money);
    });
    let staff = workbook("직원 목록", |sh, st| {
        widths(sh, &[110.0, 90.0, 110.0, 110.0]);
        put(sh, "A1", "직원 목록", st.title);
        put(sh, "A2", "부서", st.label);
        put(sh, "B2", "{{department}}", StyleId::DEFAULT);
        for (c, h) in ["이름", "직급", "입사일", "급여"].iter().enumerate() {
            put(sh, &format!("{}4", (b'A' + c as u8) as char), h, st.head);
        }
        put(sh, "A5", "{{loop employees}}{{name}}", StyleId::DEFAULT);
        put(sh, "B5", "{{title}}", StyleId::DEFAULT);
        put(sh, "C5", "{{joined}}", StyleId::DEFAULT);
        put(sh, "D5", "{{salary}}{{endloop}}", st.money);
        put(sh, "A7", "작성자", st.label);
        put(sh, "B7", "{{author}}", StyleId::DEFAULT);
    });
    vec![("quote", quote), ("statement", statement), ("staff", staff)]
}

#[test]
fn samples_survive_json_and_open_like_a_file() {
    for (name, wb) in samples() {
        let json = io::save_bytes(&wb, &format!("{name}.json")).expect("save");
        let (back, _) = io::open_bytes(&format!("{name}.json"), &json).expect("open");
        assert_eq!(back.sheets.len(), 1, "{name}");
        assert_eq!(back.sheets[0].cells.len(), wb.sheets[0].cells.len(), "{name}");
        if let Ok(dir) = std::env::var("PPM_SAMPLES_OUT") {
            std::fs::write(Path::new(&dir).join(format!("{name}.json")), &json).expect("write");
        }
    }
}

#[test]
fn every_json_in_the_directory_opens() {
    let Ok(dir) = std::env::var("PPM_SAMPLES_DIR") else { return };
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read");
        let (wb, _) = io::open_bytes("x.json", &bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // `len` is trusted on load, so it must match the cells actually there.
        for sh in &wb.sheets {
            assert_eq!(sh.cells.len(), sh.cells.iter().count(), "{}: cells.len", path.display());
        }
        n += 1;
    }
    assert!(n > 0, "no *.json in {dir}");
}
