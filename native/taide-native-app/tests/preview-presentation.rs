use std::io::{Cursor, Write};

use taide_native_app::preview_presentation::{MAX_DECODED_BYTES, MAX_SLIDES, Slide, decode};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn archive(entries: &[(&str, &[u8])], compression: CompressionMethod) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(compression);
    for (name, data) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn pptx_outline은_원본_번호순서_run_entity_empty와_비신뢰_zip_상한을_검사한다() {
    let first = br#"<!DOCTYPE p:sld [<!ENTITY outside SYSTEM "file:///not-approved">]>
        <p:sld><a:p><a:r><a:t>A &amp; </a:t></a:r><a:r><a:t>B &lt;tag&gt;</a:t></a:r></a:p>
        <a:p><a:t>&#65;&#x1d11e;&quot;&apos;&unknown;&outside;&AMP;</a:t></a:p>
        <a:p><a:t>  &#xfeff;  </a:t></a:p><a:p><a:t xml:space="preserve">ignored</a:t></a:p>
        <a:p><a:t>&amp;lt; &invalid &#bad; &&x;</a:t></a:p>
        <a:p><a:t>&#x85;</a:t></a:p></p:sld>"#;
    let expected = vec![
        Slide {
            index: 1,
            paragraphs: vec![
                "A & B <tag>".into(),
                "A\u{1d11e}\"'&unknown;&outside;&AMP;".into(),
                "&lt; &invalid &#bad; &&x;".into(),
                "\u{85}".into(),
            ],
        },
        Slide {
            index: 2,
            paragraphs: vec![" 한글 日本語 ".into()],
        },
        Slide {
            index: 3,
            paragraphs: Vec::new(),
        },
    ];
    let entries = [
        ("ppt/slides/slide10.xml", b"<p:sld/>".as_slice()),
        (
            "ppt/slides/slide2.xml",
            "<a:p><a:t> 한글 日本語 </a:t></a:p>".as_bytes(),
        ),
        ("ppt/slides/slide01.xml", first.as_slice()),
        ("ppt/slides/slide3.xml.rels", b"not XML".as_slice()),
        ("ppt/slides/../slide3.xml", b"not XML".as_slice()),
        ("../ppt/slides/slide3.xml", b"not XML".as_slice()),
        ("ppt/slides/slide-1.xml", b"not XML".as_slice()),
        ("http://outside/slides/slide4.xml", b"not XML".as_slice()),
    ];
    for compression in [CompressionMethod::Stored, CompressionMethod::Deflated] {
        let bytes = archive(&entries, compression);
        let outline = decode(&bytes).unwrap();
        assert_eq!(outline.slides, expected);
        assert!(outline.retained_bytes() < MAX_DECODED_BYTES);
        assert!(decode(&bytes[..bytes.len() / 2]).is_err());
        let mut bad_crc = bytes.clone();
        let central = bad_crc
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .unwrap();
        bad_crc[central + 16] ^= 1;
        assert!(decode(&bad_crc).is_err());
        let mut huge = bytes.clone();
        huge[central + 24..central + 28]
            .copy_from_slice(&u32::try_from(MAX_DECODED_BYTES + 1).unwrap().to_le_bytes());
        assert!(decode(&huge).is_err());
    }
    assert!(decode(b"not a ZIP").is_err());
    assert!(
        decode(&archive(
            &[("[Content_Types].xml", b"<Types/>")],
            CompressionMethod::Stored
        ))
        .is_err()
    );
    assert!(
        decode(&archive(
            &[("ppt/slides/slide1.xml", b"<a:p><a:t>&#x110000;</a:t></a:p>")],
            CompressionMethod::Stored
        ))
        .is_err()
    );
    let invalid_utf8 = archive(
        &[("ppt/slides/slide1.xml", b"<a:p><a:t>\xff</a:t></a:p>")],
        CompressionMethod::Stored,
    );
    assert_eq!(
        decode(&invalid_utf8).unwrap().slides[0].paragraphs,
        ["\u{fffd}"]
    );
    let names = (0..=MAX_SLIDES)
        .map(|index| format!("ppt/slides/slide{index}.xml"))
        .collect::<Vec<_>>();
    let entries = names
        .iter()
        .map(|name| (name.as_str(), b"<p:sld/>".as_slice()))
        .collect::<Vec<_>>();
    assert!(decode(&archive(&entries, CompressionMethod::Stored)).is_err());
}
