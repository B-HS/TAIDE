use std::io::{self, BufRead, Write};
use std::process::Command;

const OUTPUT_ROWS: usize = 1_000;
const INITIAL_OUTPUT: &str = "한𐐀e\u{301}\n\x1b]2;fixture-ready\x07";
const FINAL_OUTPUT: &str = "\x1b]52;c;c3ludGhldGlj\x07\x1b]2;fixture-finished\x07";
const CONTINUE: &str = "continue\n";

fn main() -> io::Result<()> {
    if std::env::args().len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture accepts no arguments",
        ));
    }
    let status = Command::new("/bin/stty")
        .args(["-echo", "-onlcr"])
        .status()?;
    if !status.success() {
        return Err(io::Error::other("fixture PTY setup failed"));
    }
    let mut output = io::stdout().lock();
    output.write_all(INITIAL_OUTPUT.as_bytes())?;
    output.flush()?;
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    if input != CONTINUE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected fixture command",
        ));
    }
    output.write_all(b"\x1b]133;C\x07")?;
    for row in 0..OUTPUT_ROWS {
        write!(output, "row-{row} 한e\u{301}\r\n")?;
    }
    output.write_all(FINAL_OUTPUT.as_bytes())?;
    output.flush()
}
