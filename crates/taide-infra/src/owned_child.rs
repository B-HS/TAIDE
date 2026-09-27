use std::io;
use std::process::Child;

const MIN_SIGNALABLE_PID: u32 = 2;
#[cfg(target_os = "macos")]
const PROC_PGRP_ONLY: u32 = 2;
#[cfg(target_os = "macos")]
const OWNED_GROUP_PROBE_PIDS: usize = 2;

fn signalable_child_id(child: &Child) -> io::Result<libc::pid_t> {
    if child.id() < MIN_SIGNALABLE_PID {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "child process ID cannot target a process group",
        ));
    }
    libc::pid_t::try_from(child.id()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "child process ID exceeds pid_t"))
}

/// Observes an exclusively owned direct child without reaping its PID or consuming its exit status.
/// Call only before any wait or try_wait; never reuse a cached handle after reaping.
pub fn has_exited_unreaped(child: &mut Child) -> io::Result<bool> {
    observe_child_exit(child, libc::WEXITED | libc::WNOHANG | libc::WNOWAIT)
}

/// Blocks until an exclusively owned direct child exits without reaping its PID or exit status.
/// Call only before any wait or try_wait; never reuse a cached handle after reaping.
pub fn wait_for_exit_unreaped(child: &mut Child) -> io::Result<()> {
    if observe_child_exit(child, libc::WEXITED | libc::WNOWAIT)? {
        return Ok(());
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "blocking waitid returned no child exit"))
}

fn observe_child_exit(child: &mut Child, options: libc::c_int) -> io::Result<bool> {
    let pid = signalable_child_id(child)?;
    loop {
        let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
        let result = unsafe { libc::waitid(libc::P_PID, child.id(), &mut info, options) };
        if result == 0 {
            let observed_pid = unsafe { info.si_pid() };
            if observed_pid == 0 {
                return Ok(false);
            }
            if observed_pid == pid {
                return Ok(true);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "waitid returned another child process ID",
            ));
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// Kills the group of an exclusively owned, unreaped child spawned with process_group(0).
/// Returns an error without signaling when the direct child is no longer waitable.
/// Call only before any wait or try_wait; never reuse a cached handle after reaping.
pub fn kill_unreaped_child_group(child: &mut Child) -> io::Result<()> {
    let pid = signalable_child_id(child)?;
    let has_exited = has_exited_unreaped(child)?;
    if unsafe { libc::kill(-pid, libc::SIGKILL) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    if has_exited && error.raw_os_error() == Some(libc::EPERM) && group_contains_only_child(pid, child.id())? {
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    let _ = has_exited;
    Err(error)
}

#[cfg(target_os = "macos")]
fn group_contains_only_child(pid: libc::pid_t, group_id: u32) -> io::Result<bool> {
    let mut members = [0; OWNED_GROUP_PROBE_PIDS];
    let buffer_size = libc::c_int::try_from(std::mem::size_of_val(&members))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "owned group probe buffer exceeds c_int"))?;
    let bytes = unsafe { libc::proc_listpids(PROC_PGRP_ONLY, group_id, members.as_mut_ptr().cast(), buffer_size) };
    let single_pid_bytes = libc::c_int::try_from(std::mem::size_of::<libc::pid_t>())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid_t size exceeds c_int"))?;
    Ok(bytes == single_pid_bytes && members[0] == pid)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::*;

    const FIXTURE_DURATION_SECONDS: u64 = 30;
    const FIXTURE_EXIT_CODE: i32 = 7;
    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const FIXTURE_PENDING_PROBE_MS: u64 = 60;

    struct ChildFixture(Child);

    impl Drop for ChildFixture {
        fn drop(&mut self) {
            self.0.kill().ok();
            self.0.wait().ok();
        }
    }

    #[test]
    fn blocking_종료_관찰은_살아있는_child에서_대기하고_회수와_코드를_보존한다() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "read token; exit \"$1\"", "fixture"])
            .arg(FIXTURE_EXIT_CODE.to_string())
            .env("ENV", "")
            .env("BASH_ENV", "")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut fixture = ChildFixture(command.spawn().unwrap());
        let mut stdin = fixture.0.stdin.take().unwrap();
        let (started, started_rx) = std::sync::mpsc::channel();
        let (observed, observed_rx) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            started.send(()).unwrap();
            wait_for_exit_unreaped(&mut fixture.0).unwrap();
            let still_waitable = has_exited_unreaped(&mut fixture.0).unwrap();
            observed.send(()).ok();
            (still_waitable, fixture.0.wait().unwrap().code())
        });
        let ready = started_rx.recv_timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS));
        let was_pending = observed_rx.recv_timeout(Duration::from_millis(FIXTURE_PENDING_PROBE_MS)).is_err();
        let written = stdin.write_all(b"go\n");
        drop(stdin);
        let (still_waitable, code) = waiter.join().unwrap();
        assert!(ready.is_ok());
        assert!(was_pending);
        assert!(written.is_ok());
        assert!(still_waitable);
        assert_eq!(code, Some(FIXTURE_EXIT_CODE));
    }

    #[test]
    fn 살아_있는_자기_child는_회수_없이_관찰하고_그룹을_종료한다() {
        let mut command = Command::new("sleep");
        command
            .arg(FIXTURE_DURATION_SECONDS.to_string())
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut fixture = ChildFixture(command.spawn().unwrap());
        assert!(!has_exited_unreaped(&mut fixture.0).unwrap());
        kill_unreaped_child_group(&mut fixture.0).unwrap();
        assert_eq!(fixture.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    }

    #[test]
    fn 종료_관찰과_빈_그룹_정리는_원래_종료_코드를_소비하지_않는다() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "exit \"$1\"", "fixture"])
            .arg(FIXTURE_EXIT_CODE.to_string())
            .process_group(0);
        let mut fixture = ChildFixture(command.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_millis(FIXTURE_TIMEOUT_MS);
        while !has_exited_unreaped(&mut fixture.0).unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(has_exited_unreaped(&mut fixture.0).unwrap());
        kill_unreaped_child_group(&mut fixture.0).unwrap();
        assert_eq!(fixture.0.wait().unwrap().code(), Some(FIXTURE_EXIT_CODE));
    }
}
