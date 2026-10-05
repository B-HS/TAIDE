use std::io::{self, Read, Write};
use std::process::Command;

const NORMAL_INPUT: &[u8] = "\x1b[A\x03\r\n한e\u{301}𐐀한\r日\r".as_bytes();
const APPLICATION_INPUT: &[u8] = "\x1bOA\x1b\x7f\x1b[I\x1b[O\x1b[200~한\r日\r\x1b[201~".as_bytes();
const RESET_INPUT: &[u8] = "\x1b[A한\r日\r".as_bytes();
const PHASES: &[(&str, &[u8], &[u8])] = &[
    ("normal", b"\x1b[?1l\x1b[?2004l\x1b[?1004l", NORMAL_INPUT),
    (
        "application",
        b"\x1b[?1h\x1b[?2004h\x1b[?1004h",
        APPLICATION_INPUT,
    ),
    ("reset", b"\x1b[?1l\x1b[?2004l\x1b[?1004l", RESET_INPUT),
];

fn main() -> io::Result<()> {
    if std::env::args_os().count() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture accepts no arguments",
        ));
    }
    if !Command::new("/bin/stty")
        .args(["raw", "-echo", "-onlcr"])
        .status()?
        .success()
    {
        return Err(io::Error::other("fixture PTY setup failed"));
    }
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    for (phase, modes, expected) in PHASES {
        output.write_all(modes)?;
        write!(output, "\x1b]2;input-{phase}-ready\x07")?;
        output.flush()?;
        let mut received = vec![0; expected.len()];
        input.read_exact(&mut received)?;
        if received != *expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture input bytes differ",
            ));
        }
        write!(output, "\x1b]2;input-{phase}-received\x07")?;
        output.flush()?;
    }
    output.write_all(b"\x1b]2;input-finished\x07")?;
    output.flush()
}
