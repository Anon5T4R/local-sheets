//! A workbook written by another application: opened, saved to a temporary `.xlsx` file, read back
//! from disk and compared whole. Set `LOCALSHEETS_XLSX_IN=<file>` to point it at one; the corpus
//! lives outside the repository, so CI skips it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use localsheets_model::Workbook;

fn temp_file(extension: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("localsheets-real-file-{}-{n}.{extension}", std::process::id()))
}

/// A workbook that had no `<fileVersion>` gets `<fileVersion appName="localsheets"/>` on its first
/// save (`crates/xlsx/src/write.rs`, #1699) and a read keeps it: dropping exactly that element, and
/// nothing else, lets the reopened workbook be compared with the one that was opened.
fn without_the_file_version_its_first_save_added(mut wb: Workbook) -> Workbook {
    let Some(p) = wb.preserved.as_deref() else { return wb };
    let only_the_added_element = p.package_rels.is_empty()
        && p.rels.is_empty()
        && p.parts.is_empty()
        && p.vml.is_none()
        && p.sheets.is_empty()
        && p.elements.len() == 1
        && p.elements[0].slot == "fileVersion"
        && p.elements[0].xml == r#"<fileVersion appName="localsheets"/>"#;
    if only_the_added_element {
        wb.preserved = None;
    }
    wb
}

#[test]
fn real_file_opens_saves_to_a_temporary_workbook_and_reopens() {
    let Ok(path) = std::env::var("LOCALSHEETS_XLSX_IN") else { return };
    let xlsx = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (opened, report) = localsheets_xlsx::read_xlsx(&xlsx).expect("read the real file");
    assert!(!opened.sheets.is_empty(), "the real file opened without sheets: {report:?}");

    let book = temp_file("xlsx");
    std::fs::write(&book, localsheets_xlsx::write_xlsx(&opened).expect("write the workbook"))
        .unwrap_or_else(|e| panic!("write {}: {e}", book.display()));
    let bytes = std::fs::read(&book).unwrap_or_else(|e| panic!("read {}: {e}", book.display()));
    std::fs::remove_file(&book).expect("remove the temporary workbook");

    let (reopened, _) = localsheets_xlsx::read_xlsx(&bytes).expect("reopen the saved workbook");
    assert_eq!(
        without_the_file_version_its_first_save_added(reopened),
        without_the_file_version_its_first_save_added(opened),
        "the saved workbook must hold everything the opened one had"
    );
}
