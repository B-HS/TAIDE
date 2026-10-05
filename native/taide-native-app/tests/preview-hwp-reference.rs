use std::io::{Cursor, Write};

use taide_native_app::preview_hwp_preflight::admit;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn hwpx_manifest가_참조하는_비표준_확장자도_xml_검사를_통과해야_한다() {
    for path in [
        "Contents/section0.dat",
        "Contents/section&amp;x.dat",
        "Contents/section\nx.dat",
    ] {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let manifest = format!(
            "<package><item id='s0' href='{path}' media-type='application/xml'/></package>"
        );
        for (name, content) in [
            (
                path,
                "<!DOCTYPE a [<!ENTITY external SYSTEM 'file:///not-approved'>]><a>&external;</a>",
            ),
            ("Contents/content.hpf", &manifest),
        ] {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        assert!(
            admit(&bytes).unwrap_err().to_string().contains("DTD"),
            "{path}"
        );
    }
}
