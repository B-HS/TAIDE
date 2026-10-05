use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use taide_model::file::READ_ONLY_FILE_BYTES;
use taide_native_app::{
    preview_web_document::source_url,
    preview_web_helper::{FLAG, MAX_SOURCE_BYTES, REPLY_MAGIC, REQUEST_MAGIC},
};

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const SOURCE: &str = "/synthetic project/pages/index.html";
const EXPECTED_PREFIX: &str =
    "<!doctype html><html><head><meta http-equiv=\"Content-Security-Policy\"";

#[test]
fn html_helper는_gui_bootstrap없이_독립_process에서_제한된_frame을_처리한다() {
    let source = source_url(Path::new(SOURCE)).unwrap();
    let bytes = b"<base href='../assets/'><p>synthetic <img src='photo.png'>";
    let mut child = Command::new(EXECUTABLE)
        .arg(FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(&REQUEST_MAGIC).unwrap();
    input
        .write_all(&u32::try_from(source.as_str().len()).unwrap().to_le_bytes())
        .unwrap();
    input
        .write_all(&u32::try_from(bytes.len()).unwrap().to_le_bytes())
        .unwrap();
    input.write_all(source.as_str().as_bytes()).unwrap();
    input.write_all(bytes).unwrap();
    drop(input);
    let mut output = child.stdout.take().unwrap();
    let mut magic = [0; REPLY_MAGIC.len()];
    output.read_exact(&mut magic).unwrap();
    assert_eq!(magic, REPLY_MAGIC);
    let mut length = [0; size_of::<u32>()];
    output.read_exact(&mut length).unwrap();
    let length = usize::try_from(u32::from_le_bytes(length)).unwrap();
    assert!(length <= taide_native_app::preview_web_document::MAX_OUTPUT_BYTES);
    let mut body = vec![0; length];
    output.read_exact(&mut body).unwrap();
    let body = String::from_utf8(body).unwrap();
    assert!(body.starts_with(EXPECTED_PREFIX));
    assert!(body.contains("taide-preview://localhost/synthetic%20project/assets/"));
    assert!(body.contains("<img src=\"photo.png\">"));
    let mut trailing = [0; 1];
    assert_eq!(output.read(&mut trailing).unwrap(), 0);
    assert!(child.wait().unwrap().success());

    for (source_length, document_length) in [
        (MAX_SOURCE_BYTES + 1, 0),
        (0, u32::try_from(READ_ONLY_FILE_BYTES).unwrap() + 1),
    ] {
        let mut child = Command::new(EXECUTABLE)
            .arg(FLAG)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        input.write_all(&REQUEST_MAGIC).unwrap();
        input.write_all(&source_length.to_le_bytes()).unwrap();
        input.write_all(&document_length.to_le_bytes()).unwrap();
        drop(input);
        let result = child.wait_with_output().unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains("input budget")
        );
    }
    let result = Command::new(EXECUTABLE)
        .args([FLAG, "unexpected"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("additional arguments")
    );
}
