//! The sample templates of the PPM `/alternate-spreadjs` test page (JSON workbooks: forms with labels, empty input cells and a few formulas).
//!
//! `PPM_SAMPLES_OUT=<dir> cargo test -p gridcraft-engine --test spreadjs_samples` writes them as `<dir>/<name>.json`.
//! `PPM_SAMPLES_DIR=<dir>` makes `every_json_in_the_directory_opens` open each `*.json` there the way the app does,
//! which also checks workbooks the page filled in.

use std::path::Path;

use gridcraft_core::{CellRef, Value};
use gridcraft_engine::io;
use gridcraft_model::{Cell, Color, Fill, Formula, LineInfo, Sheet, StyleId, Workbook};

struct Styles {
    title: StyleId,
    label: StyleId,
    head: StyleId,
    /// An input cell: light yellow, text.
    input: StyleId,
    /// An input cell for amounts.
    money: StyleId,
    /// A computed amount.
    total: StyleId,
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
    let input = wb.styles.derive(normal, |s| s.fill = Fill::solid(Color::Rgb(0xFFF9DB)));
    let money = wb.styles.derive(input, |s| s.num_fmt = gridcraft_model::NumFmt::new("#,##0"));
    let total = wb.styles.derive(normal, |s| s.num_fmt = gridcraft_model::NumFmt::new("#,##0"));
    Styles { title, label, head, input, money, total }
}

fn cell_ref(at: &str) -> CellRef {
    CellRef::parse(at).unwrap_or_else(|| panic!("bad cell {at}"))
}

fn put(sh: &mut Sheet, at: &str, text: &str, style: StyleId) {
    sh.cells.set(cell_ref(at), Cell { value: Value::Text(text.into()), formula: None, style });
}

/// An empty, styled cell the form's user fills in.
fn input(sh: &mut Sheet, at: &str, style: StyleId) {
    sh.cells.set(cell_ref(at), Cell { value: Value::Empty, formula: None, style });
}

/// A formula cell (its value is calculated when the workbook opens).
fn formula(sh: &mut Sheet, at: &str, text: &str, style: StyleId) {
    sh.cells.set(cell_ref(at), Cell { value: Value::Empty, formula: Some(std::sync::Arc::new(Formula::new(text))), style });
}

fn widths(sh: &mut Sheet, widths: &[f32]) {
    for (col, w) in widths.iter().enumerate() {
        sh.cols.insert(col as u32, LineInfo { size: Some(*w), ..LineInfo::default() });
    }
}

fn heads(sh: &mut Sheet, row: u32, names: &[&str], style: StyleId) {
    for (c, h) in names.iter().enumerate() {
        put(sh, &format!("{}{row}", (b'A' + c as u8) as char), h, style);
    }
}

fn workbook(names: &[&str], build: impl FnOnce(&mut [Sheet], &Styles)) -> Workbook {
    let mut wb = Workbook::new();
    let st = styles(&mut wb);
    let mut sheets: Vec<Sheet> = names.iter().map(|n| Sheet::new(*n)).collect();
    build(&mut sheets, &st);
    wb.sheets = sheets.into_iter().map(std::sync::Arc::new).collect();
    wb
}

/// (file name, workbook): forms with labels and empty input cells.
fn samples() -> Vec<(&'static str, Workbook)> {
    let quote = workbook(&["견적서"], |s, st| {
        let sh = &mut s[0];
        widths(sh, &[180.0, 70.0, 100.0, 110.0]);
        put(sh, "A1", "견적서", st.title);
        put(sh, "A3", "고객사", st.label);
        input(sh, "B3", st.input);
        put(sh, "A4", "발행일", st.label);
        input(sh, "B4", st.input);
        heads(sh, 6, &["품목", "수량", "단가", "금액"], st.head);
        for r in 7..=9 {
            input(sh, &format!("A{r}"), st.input);
            input(sh, &format!("B{r}"), st.money);
            input(sh, &format!("C{r}"), st.money);
            formula(sh, &format!("D{r}"), &format!("B{r}*C{r}"), st.total);
        }
        put(sh, "A11", "합계", st.label);
        formula(sh, "D11", "SUM(D7:D9)", st.total);
    });
    let statement = workbook(&["거래명세서"], |s, st| {
        let sh = &mut s[0];
        widths(sh, &[160.0, 70.0, 100.0, 110.0, 160.0]);
        put(sh, "A1", "거래명세서", st.title);
        put(sh, "A3", "거래처", st.label);
        input(sh, "B3", st.input);
        put(sh, "A4", "거래일", st.label);
        input(sh, "B4", st.input);
        heads(sh, 6, &["품명", "수량", "단가", "공급가액", "비고"], st.head);
        for r in 7..=10 {
            input(sh, &format!("A{r}"), st.input);
            input(sh, &format!("B{r}"), st.money);
            input(sh, &format!("C{r}"), st.money);
            formula(sh, &format!("D{r}"), &format!("B{r}*C{r}"), st.total);
            input(sh, &format!("E{r}"), st.input);
        }
        put(sh, "A12", "공급가액 합계", st.label);
        formula(sh, "D12", "SUM(D7:D10)", st.total);
        put(sh, "A13", "부가세(10%)", st.label);
        formula(sh, "D13", "D12*0.1", st.total);
    });
    let staff = workbook(&["직원 목록", "비고"], |s, st| {
        let sh = &mut s[0];
        widths(sh, &[110.0, 90.0, 110.0, 110.0]);
        put(sh, "A1", "직원 목록", st.title);
        put(sh, "A2", "부서", st.label);
        input(sh, "B2", st.input);
        heads(sh, 4, &["이름", "직급", "입사일", "급여"], st.head);
        for r in 5..=8 {
            for c in ["A", "B", "C"] {
                input(sh, &format!("{c}{r}"), st.input);
            }
            input(sh, &format!("D{r}"), st.money);
        }
        put(sh, "A10", "급여 합계", st.label);
        formula(sh, "D10", "SUM(D5:D8)", st.total);
        let memo = &mut s[1];
        widths(memo, &[80.0, 300.0]);
        put(memo, "A1", "메모", st.label);
        input(memo, "B1", st.input);
    });
    vec![("quote", quote), ("statement", statement), ("staff", staff)]
}

#[test]
fn samples_survive_json_and_open_like_a_file() {
    for (name, wb) in samples() {
        let json = io::save_bytes(&wb, &format!("{name}.json")).expect("save");
        let (back, _) = io::open_bytes(&format!("{name}.json"), &json).expect("open");
        assert_eq!(back.sheets.len(), wb.sheets.len(), "{name}");
        for (a, b) in back.sheets.iter().zip(&wb.sheets) {
            assert_eq!(a.cells.len(), b.cells.len(), "{name}");
        }
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

/// The page writes formulas as `{text, array}` with no cached value; opening recalculates them.
#[test]
fn a_formula_the_page_wrote_calculates_on_open() {
    // Through JSON, as the page's workbook arrives: formulas have no parsed expression yet.
    let json = io::save_bytes(&Workbook::new(), "x.json").expect("save");
    let (mut wb, _) = io::open_bytes("x.json", &json).expect("open");
    let sh = std::sync::Arc::make_mut(&mut wb.sheets[0]);
    sh.cells.set(cell_ref("A1"), Cell::value(Value::Number(4.0)));
    sh.cells.set(
        cell_ref("B1"),
        Cell {
            value: Value::Empty,
            formula: Some(std::sync::Arc::new(Formula { text: "A1*2".into(), expr: None, array: None })),
            style: StyleId::DEFAULT,
        },
    );
    gridcraft_calc::Calc::new().recalc_all(&mut wb);
    assert_eq!(wb.sheets[0].cells.get(cell_ref("B1")).map(|c| c.value.clone()), Some(Value::Number(8.0)));
}
