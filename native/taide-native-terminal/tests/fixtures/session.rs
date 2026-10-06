use std::io::{self, BufRead, Read, Write};
use std::process::Command;

const OUTPUT_ROWS: usize = 1000;
const FLOOD_ENV: &str = "TAIDE_NATIVE_FIXTURE_FLOOD";
const FLOOD_ROWS: usize = 60_000;
const INITIAL_OUTPUT: &str = "한𐐀e\u{301}\n\x1b]2;native-ready\x07";
const LINKS_ENV: &str = "TAIDE_NATIVE_FIXTURE_LINKS";
const COMMANDS_ENV: &str = "TAIDE_NATIVE_FIXTURE_COMMANDS";
const COMMAND_ROWS: usize = 40;
const FILE_LINKS_ENV: &str = "TAIDE_NATIVE_FIXTURE_FILE_LINKS";
const FILE_LINKS_OUTPUT: &str = "汉𐐀e\u{301} ../other/editor.rs:2:3 ghost.rs:7\r\n";
const LINKS_OUTPUT: &str = "\x1b]8;id=synthetic;https://example.com/target\x1b\\한𐐀e\u{301} link\x1b]8;;\x1b\\\r\nhttps://example.com/plain\r\n";
const FINAL_OUTPUT: &str = "\x1b[?2026hfinal-sync\x1b]2;native-finished\x07";
const CONTINUE: &str = "continue\n";
const FINAL_QUERY_ENV: &str = "TAIDE_NATIVE_FIXTURE_FINAL_QUERY";
const FINAL_QUERY: &[u8] = b"\x1b]4;1;?\x07\x1b[14t";
const LIVE_SYNC_ENV: &str = "TAIDE_NATIVE_FIXTURE_LIVE_SYNC";
const LIVE_SYNC_INPUT: &str = "sync\n";
const LIVE_SYNC_OUTPUT: &[u8] = b"\x1b[?2026hlive-sync\x1b]2;native-live\x07";
const WHEEL_ENV: &str = "TAIDE_NATIVE_FIXTURE_WHEEL";
const WHEEL_OPEN: &[u8] = b"\x1b[?1049h\x1b[?1h";
const WHEEL_INPUT: &[u8; 9] = b"a\x1bOAb\x1bOB\t";
const WHEEL_FINISHED: &[u8] = b"\x1b[?1049l\x1b]2;native-wheel-finished\x07";
const MOUSE_ENV: &str = "TAIDE_NATIVE_FIXTURE_MOUSE";
const MOUSE_OPEN: &[u8] = b"\x1b[?1000h";
const MOUSE_BINARY_INPUT: &[u8; 12] = b"\x1b[M \xff!\x1b[M#\xff!";
const MOUSE_SGR_READY: &[u8] = b"\x1b[?1002h\x1b[?1006h\x1b]2;native-mouse-sgr-ready\x07";
const MOUSE_SGR_INPUT: &[u8] = b"a\x1b[<29;3;2Mb\x1b[<29;3;2m\x1b[<64;3;2Mcd";
const MOUSE_FINISHED: &[u8] = b"\x1b[?1002l\x1b[?1006l\x1b]2;native-mouse-finished\x07";
const HIDDEN_MOUSE_ENV: &str = "TAIDE_NATIVE_FIXTURE_HIDDEN_MOUSE";
const HIDDEN_MOUSE_FILL: usize = 63;
const HIDDEN_MOUSE_REPORTS: &[u8] = b"\x1b[<0;3;2M\x1b[<0;3;2m";
const HIDDEN_MOUSE_CYCLE: &[u8] = b"\x1b]2;native-hidden-cycle-ready\x07";
const HIDDEN_MOUSE_SWITCH: &[u8] = b"\x1b[<0;3;2M\x1b[<0;3;2mz";
const FOCUS_ENV: &str = "TAIDE_NATIVE_FIXTURE_FOCUS";
const FOCUS_OPEN: &[u8] = b"\x1b[?1004h";
const FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[I\x1b[O\x1b[I\x1b[O\x1b[Ix\x1b[O\x1b[I\x1b[O\x1b[I\x1b[O";
const AX_FOCUS_ENV: &str = "TAIDE_NATIVE_FIXTURE_AX_FOCUS";
const AX_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1b[O\x1b[Itinue\n";
const AX_INITIAL_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[I\x1b[O\x1b[Icontinue\n";
const AX_NAVIGATION_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\t\x1b[D\x1b[O\x1b[Itinue\n";
const AX_POINTER_OPEN: &[u8] = b"\x1b[?1003h\x1b[?1006h";
const AX_POINTER_FOCUS_INPUT: &[u8] =
    b"\x1b[O\x1b[Icon\x1b[<35;1;1M\x1b[<64;1;1M\x1b[O\x1b[Itinue\n";
const AX_ALT_WHEEL_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1bOA\x1bOB\x1b[O\x1b[Itinue\n";
const PRIMARY_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1b[O\x1b[I\x1b[<0;1;1Mtinue\n\x1b[<0;1;1m";
const MIDDLE_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1b[O\x1b[I\x1b[<1;1;1Mtinue\n\x1b[<1;1;1m";
const POINTER_THEN_AX_FOCUS_INPUT: &[u8] =
    b"\x1b[O\x1b[Icon\x1b[O\x1b[I\x1b[<0;1;1Mtinue\n\x1b[<0;1;1m\x1b[O\x1b[I";
const MENU_KEY_ENV: &str = "TAIDE_NATIVE_FIXTURE_MENU_KEY";
const MENU_KEY_INPUT: &[u8] = b"con\x1b[21;2~tinue\n";
const MENU_FOCUS_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1b[O\x1b[Itinue\n";
const MENU_MOUSE_OPEN: &[u8] = b"\x1b[?1000h\x1b[?1006h";
const MENU_MOUSE_INPUT: &[u8] = b"\x1b[O\x1b[Icon\x1b[<2;1;1M\x1b[O\x1b[<2;1;1m\x1b[Itinue\n";
const FOCUS_FINISHED: &[u8] = b"\x1b[?1004l\x1b]2;native-focus-finished\x07";
const FOCUS_PRESSURE_ENV: &str = "TAIDE_NATIVE_FIXTURE_FOCUS_PRESSURE";
const FOCUS_PRESSURE_FILL: usize = 63;
const FOCUS_PRESSURE_PREFIX: &[u8] = b"\x1b[O\x1b[I";
const FOCUS_PRESSURE_REPORTS: &[u8] = b"\x1b[O\x1b[I\x1b[O\x1b[I";
const FOCUS_PRESSURE_READY: &[u8] = b"\x1b]2;native-focus-pressure-ready\x07";
const FOCUS_PRESSURE_END: &[u8] = b"z\x1b[O";
const QUERY_ORDER_ENV: &str = "TAIDE_NATIVE_FIXTURE_QUERY_ORDER";
const QUERY_ORDER_OUTPUT: &[u8] = b"\x1b]2;native-query-issued\x07\x1b[5n";
const QUERY_ORDER_INPUT: &[u8] = b"x\x1b[0nz";
const HIDDEN_WHEEL_ENV: &str = "TAIDE_NATIVE_FIXTURE_HIDDEN_WHEEL";
const HIDDEN_WHEEL_FILL: usize = 64;
const HIDDEN_WHEEL_UP: &[u8] = b"\x1bOA";
const HIDDEN_WHEEL_READY: &[u8] = b"\x1b]2;native-hidden-wheel-ready\x07";
const HIDDEN_WHEEL_END: &[u8] = b"\x1bOBz";

fn main() -> io::Result<()> {
    if std::env::args_os().count() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture accepts no arguments",
        ));
    }
    let is_wheel = std::env::var_os(WHEEL_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let is_mouse = std::env::var_os(MOUSE_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let is_hidden_mouse =
        std::env::var_os(HIDDEN_MOUSE_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let is_focus = std::env::var_os(FOCUS_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let ax_focus_mode = std::env::var_os(AX_FOCUS_ENV);
    let is_initial_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("2"));
    let is_navigation_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("3"));
    let is_pointer_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("4"));
    let is_alt_wheel_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("5"));
    let is_primary_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("6"));
    let is_middle_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("7"));
    let is_pointer_then_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("8"));
    let is_ax_focus = ax_focus_mode.as_deref() == Some(std::ffi::OsStr::new("1"))
        || is_initial_ax_focus
        || is_navigation_ax_focus
        || is_pointer_ax_focus
        || is_alt_wheel_ax_focus
        || is_primary_focus
        || is_middle_focus
        || is_pointer_then_ax_focus;
    let is_focus_pressure =
        std::env::var_os(FOCUS_PRESSURE_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let is_query_order =
        std::env::var_os(QUERY_ORDER_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let is_hidden_wheel =
        std::env::var_os(HIDDEN_WHEEL_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let menu_key_mode = std::env::var_os(MENU_KEY_ENV);
    let is_menu_prefix = menu_key_mode.as_deref() == Some(std::ffi::OsStr::new("2"));
    let is_menu_mouse = menu_key_mode.as_deref() == Some(std::ffi::OsStr::new("4"));
    let is_menu_focus =
        menu_key_mode.as_deref() == Some(std::ffi::OsStr::new("3")) || is_menu_mouse;
    let is_menu_key = menu_key_mode.as_deref() == Some(std::ffi::OsStr::new("1"))
        || is_menu_prefix
        || is_menu_focus;
    if [
        is_wheel,
        is_mouse,
        is_hidden_mouse,
        is_focus,
        is_ax_focus,
        is_focus_pressure,
        is_query_order,
        is_hidden_wheel,
        is_menu_key,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture input modes are mutually exclusive",
        ));
    }
    let mut setup = Command::new("/bin/stty");
    setup.args(["-echo", "-onlcr"]);
    if is_wheel
        || is_mouse
        || is_hidden_mouse
        || is_focus
        || is_ax_focus
        || is_focus_pressure
        || is_query_order
        || is_hidden_wheel
        || is_menu_key
    {
        setup.args(["-icanon", "min", "1", "time", "0"]);
    }
    if is_mouse {
        setup.arg("-istrip");
    }
    if !setup.status()?.success() {
        return Err(io::Error::other("fixture PTY setup failed"));
    }
    let mut output = io::stdout().lock();
    if is_wheel || is_hidden_wheel {
        output.write_all(WHEEL_OPEN)?;
    }
    if is_mouse {
        output.write_all(MOUSE_OPEN)?;
    }
    if is_hidden_mouse {
        output.write_all(MOUSE_SGR_READY)?;
    }
    if is_focus || is_focus_pressure || is_ax_focus || is_menu_focus {
        output.write_all(FOCUS_OPEN)?;
    }
    if is_menu_mouse {
        output.write_all(MENU_MOUSE_OPEN)?;
    }
    if is_pointer_ax_focus || is_primary_focus || is_middle_focus || is_pointer_then_ax_focus {
        output.write_all(AX_POINTER_OPEN)?;
    }
    if is_alt_wheel_ax_focus {
        output.write_all(WHEEL_OPEN)?;
    }
    if std::env::var_os(LINKS_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        output.write_all(LINKS_OUTPUT.as_bytes())?;
    }
    if std::env::var_os(FILE_LINKS_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        output.write_all(FILE_LINKS_OUTPUT.as_bytes())?;
    }
    if std::env::var_os(COMMANDS_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        for index in 0..COMMAND_ROWS {
            write!(
                output,
                "\x1b]133;A\x07\x1b]133;C\x07command-{index}\x1b]133;D;{}\x07\r\n",
                index % 2
            )?;
        }
    }
    output.write_all(INITIAL_OUTPUT.as_bytes())?;
    output.flush()?;
    if is_hidden_wheel {
        let mut fill = [0; HIDDEN_WHEEL_FILL];
        io::stdin().lock().read_exact(&mut fill)?;
        let mut up = [0; HIDDEN_WHEEL_UP.len()];
        io::stdin().lock().read_exact(&mut up)?;
        if fill != [b'x'; HIDDEN_WHEEL_FILL] || up != HIDDEN_WHEEL_UP {
            return Err(io::Error::other("unexpected hidden wheel fixture input"));
        }
        output.write_all(HIDDEN_WHEEL_READY)?;
        output.flush()?;
        let mut end = [0; HIDDEN_WHEEL_END.len()];
        io::stdin().lock().read_exact(&mut end)?;
        if end != HIDDEN_WHEEL_END {
            return Err(io::Error::other("hidden wheel was overtaken"));
        }
        output.write_all(WHEEL_FINISHED)?;
        return output.flush();
    }
    if is_query_order {
        let mut start = [0];
        io::stdin().lock().read_exact(&mut start)?;
        if start != *b"s" {
            return Err(io::Error::other("unexpected query order start"));
        }
        output.write_all(QUERY_ORDER_OUTPUT)?;
        output.flush()?;
        let mut input = [0; QUERY_ORDER_INPUT.len()];
        io::stdin().lock().read_exact(&mut input)?;
        if input != QUERY_ORDER_INPUT {
            return Err(io::Error::other("query overtook previously queued input"));
        }
        output.write_all(FINAL_OUTPUT.as_bytes())?;
        return output.flush();
    }
    if is_focus_pressure {
        let mut prefix = [0; FOCUS_PRESSURE_PREFIX.len()];
        io::stdin().lock().read_exact(&mut prefix)?;
        let mut fill = [0; FOCUS_PRESSURE_FILL];
        io::stdin().lock().read_exact(&mut fill)?;
        let mut reports = [0; FOCUS_PRESSURE_REPORTS.len()];
        io::stdin().lock().read_exact(&mut reports)?;
        if prefix != FOCUS_PRESSURE_PREFIX
            || fill != [b'x'; FOCUS_PRESSURE_FILL]
            || reports != FOCUS_PRESSURE_REPORTS
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "focus pressure input was lost or overtaken",
            ));
        }
        output.write_all(FOCUS_PRESSURE_READY)?;
        output.flush()?;
        let mut end = [0; FOCUS_PRESSURE_END.len()];
        io::stdin().lock().read_exact(&mut end)?;
        if end != FOCUS_PRESSURE_END {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "focus pressure tail was reordered",
            ));
        }
        output.write_all(FOCUS_FINISHED)?;
        return output.flush();
    }
    if is_menu_key {
        let expected = if is_menu_mouse {
            MENU_MOUSE_INPUT
        } else if is_menu_focus {
            MENU_FOCUS_INPUT
        } else if is_menu_prefix {
            CONTINUE.as_bytes()
        } else {
            MENU_KEY_INPUT
        };
        let mut input = vec![0; expected.len()];
        io::stdin().lock().read_exact(&mut input)?;
        if input != expected {
            return Err(io::Error::other("terminal menu key replaced source input"));
        }
        return output.flush();
    }
    if is_ax_focus {
        let expected = if is_pointer_then_ax_focus {
            POINTER_THEN_AX_FOCUS_INPUT
        } else if is_primary_focus {
            PRIMARY_FOCUS_INPUT
        } else if is_middle_focus {
            MIDDLE_FOCUS_INPUT
        } else if is_alt_wheel_ax_focus {
            AX_ALT_WHEEL_FOCUS_INPUT
        } else if is_pointer_ax_focus {
            AX_POINTER_FOCUS_INPUT
        } else if is_navigation_ax_focus {
            AX_NAVIGATION_FOCUS_INPUT
        } else if is_initial_ax_focus {
            AX_INITIAL_FOCUS_INPUT
        } else {
            AX_FOCUS_INPUT
        };
        let mut input = vec![0; expected.len()];
        io::stdin().lock().read_exact(&mut input)?;
        if input != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "accessibility focus input was reordered",
            ));
        }
        output.write_all(FOCUS_FINISHED)?;
        return output.flush();
    }
    if is_focus {
        let mut input = [0; FOCUS_INPUT.len()];
        io::stdin().lock().read_exact(&mut input)?;
        if input != FOCUS_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected focus fixture input",
            ));
        }
        output.write_all(FOCUS_FINISHED)?;
        return output.flush();
    }
    if is_hidden_mouse {
        let mut fill = [0; HIDDEN_MOUSE_FILL];
        io::stdin().lock().read_exact(&mut fill)?;
        let mut reports = [0; HIDDEN_MOUSE_REPORTS.len()];
        io::stdin().lock().read_exact(&mut reports)?;
        if fill != [b'x'; HIDDEN_MOUSE_FILL] || reports != HIDDEN_MOUSE_REPORTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected hidden mouse fixture input",
            ));
        }
        output.write_all(HIDDEN_MOUSE_CYCLE)?;
        output.flush()?;
        let mut switched = [0; HIDDEN_MOUSE_SWITCH.len()];
        io::stdin().lock().read_exact(&mut switched)?;
        if switched != HIDDEN_MOUSE_SWITCH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "hidden mouse release was overtaken",
            ));
        }
        output.write_all(MOUSE_FINISHED)?;
        return output.flush();
    }
    if is_mouse {
        let mut binary = [0; MOUSE_BINARY_INPUT.len()];
        io::stdin().lock().read_exact(&mut binary)?;
        if binary != *MOUSE_BINARY_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected binary mouse fixture input",
            ));
        }
        output.write_all(MOUSE_SGR_READY)?;
        output.flush()?;
        let mut sgr = [0; MOUSE_SGR_INPUT.len()];
        io::stdin().lock().read_exact(&mut sgr)?;
        if sgr != MOUSE_SGR_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected SGR mouse fixture input",
            ));
        }
        output.write_all(MOUSE_FINISHED)?;
        return output.flush();
    }
    if is_wheel {
        let mut input = [0; WHEEL_INPUT.len()];
        io::stdin().lock().read_exact(&mut input)?;
        if input != *WHEEL_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected wheel fixture input",
            ));
        }
        output.write_all(WHEEL_FINISHED)?;
        return output.flush();
    }
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    if std::env::var_os(LIVE_SYNC_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        if input != LIVE_SYNC_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected live sync fixture command",
            ));
        }
        output.write_all(LIVE_SYNC_OUTPUT)?;
        output.flush()?;
        input.clear();
        io::stdin().lock().read_line(&mut input)?;
    }
    if input != CONTINUE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected fixture command",
        ));
    }
    if std::env::var_os(FLOOD_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        let mut flood = Vec::new();
        for row in 0..FLOOD_ROWS {
            write!(flood, "row-{row} 한e\u{301}\r\n")?;
        }
        output.write_all(&flood)?;
    } else {
        for row in 0..OUTPUT_ROWS {
            write!(output, "row-{row} 한e\u{301}\r\n")?;
        }
    }
    output.write_all(FINAL_OUTPUT.as_bytes())?;
    if std::env::var_os(FINAL_QUERY_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        output.write_all(FINAL_QUERY)?;
    }
    output.flush()
}
