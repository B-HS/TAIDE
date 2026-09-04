use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// Environment variable that turns instrumentation on or off for the whole process. Accepts
/// `1`/`true`/`on` and `0`/`false`/`off` (ASCII case-insensitive); anything else — including the
/// variable being absent — falls back to the build default, which is on for `debug_assertions`
/// builds and off for release ones. See [`resolve_enabled`].
pub const PERF_ENV_VAR: &str = "TAIDE_PERF";

const TRUTHY_TOKENS: [&str; 3] = ["1", "true", "on"];
const FALSY_TOKENS: [&str; 3] = ["0", "false", "off"];

/// Decides the gate value from a raw `TAIDE_PERF` reading. Split out from [`init`] so the parsing
/// rule is testable without touching the process environment (which is global and would race the
/// rest of the test binary).
pub fn resolve_enabled(raw: Option<&str>, debug_build: bool) -> bool {
    let Some(value) = raw.map(str::trim) else {
        return debug_build;
    };
    if TRUTHY_TOKENS.iter().any(|token| value.eq_ignore_ascii_case(token)) {
        return true;
    }
    if FALSY_TOKENS.iter().any(|token| value.eq_ignore_ascii_case(token)) {
        return false;
    }
    debug_build
}

/// A duration slot — one wall-clock measurement site, timed by [`Span`]'s RAII drop.
///
/// Slots are a **closed, compile-time set** rather than free-form strings on purpose: the registry
/// stores one fixed array entry per variant and reaches it by `self as usize`, so recording is a
/// plain atomic add with no hashing, no allocation, and — most importantly — no shared lock that
/// parallel workers (search, git, the pty reader) would queue behind. `PerfRegistry`'s own doc
/// spells out that trade-off.
///
/// Adding a variant means adding it to [`SpanSlot::ALL`] and [`SpanSlot::name`] as well; the
/// `모든_span_슬롯은_선언_순서와_같은_인덱스를_가진다` test fails otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanSlot {
    SetupMainWindow,
    SetupLocaleWarm,
    SetupStateRestore,
    SetupDeferredRestore,
    ProjectOpen,
    ProjectActivate,
    FileOpen,
    TreeToggle,
    TreeReveal,
    GitStatus,
    SearchRun,
    SearchListFiles,
}

impl SpanSlot {
    /// Declaration order, which is also snapshot order and index order.
    pub const ALL: &'static [SpanSlot] = &[
        SpanSlot::SetupMainWindow,
        SpanSlot::SetupLocaleWarm,
        SpanSlot::SetupStateRestore,
        SpanSlot::SetupDeferredRestore,
        SpanSlot::ProjectOpen,
        SpanSlot::ProjectActivate,
        SpanSlot::FileOpen,
        SpanSlot::TreeToggle,
        SpanSlot::TreeReveal,
        SpanSlot::GitStatus,
        SpanSlot::SearchRun,
        SpanSlot::SearchListFiles,
    ];

    /// Stable wire name — what `perf_snapshot` reports and what `docs/debugging.md` documents.
    /// Renaming one breaks every recorded baseline comparison, so treat these as a contract.
    pub const fn name(self) -> &'static str {
        match self {
            SpanSlot::SetupMainWindow => "setup.main_window",
            SpanSlot::SetupLocaleWarm => "setup.locale_warm",
            SpanSlot::SetupStateRestore => "setup.state_restore",
            SpanSlot::SetupDeferredRestore => "setup.deferred_restore",
            SpanSlot::ProjectOpen => "project_open",
            SpanSlot::ProjectActivate => "project_activate",
            SpanSlot::FileOpen => "file_open",
            SpanSlot::TreeToggle => "tree_toggle",
            SpanSlot::TreeReveal => "tree_reveal",
            SpanSlot::GitStatus => "git_status",
            SpanSlot::SearchRun => "search_run",
            SpanSlot::SearchListFiles => "search_list_files",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// An accumulating counter slot — a running total with no duration attached.
///
/// This is the shape every **high-frequency** site must use. A [`Span`] on the pty reader loop or
/// on `lsp_send` would pay `Instant::now()` twice per chunk/message and pollute the very number it
/// is trying to measure (research 3b §7 risk "계측 오버헤드가 측정 대상을 왜곡"), so those sites
/// add to a counter instead and the throughput is derived from wall time outside the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterSlot {
    PtyOutputBytes,
    PtyOutputChunks,
    LspSend,
    /// Invocations whose command name was not in the table [`PerfRegistry::install_commands`]
    /// received — a Tauri plugin command, or an app command added without regenerating the
    /// dispatch table. Without this slot such calls would vanish silently.
    UnlistedCommand,
}

impl CounterSlot {
    /// Declaration order, which is also snapshot order and index order.
    pub const ALL: &'static [CounterSlot] = &[
        CounterSlot::PtyOutputBytes,
        CounterSlot::PtyOutputChunks,
        CounterSlot::LspSend,
        CounterSlot::UnlistedCommand,
    ];

    /// Stable wire name — see [`SpanSlot::name`].
    pub const fn name(self) -> &'static str {
        match self {
            CounterSlot::PtyOutputBytes => "pty.output_bytes",
            CounterSlot::PtyOutputChunks => "pty.output_chunks",
            CounterSlot::LspSend => "lsp_send",
            CounterSlot::UnlistedCommand => "command.unlisted",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

pub const SPAN_SLOT_COUNT: usize = SpanSlot::ALL.len();
pub const COUNTER_SLOT_COUNT: usize = CounterSlot::ALL.len();

/// One duration slot's accumulated readout, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpanSample {
    pub count: u64,
    pub total_ns: u64,
    pub max_ns: u64,
}

#[derive(Debug, Default)]
struct SpanStats {
    count: AtomicU64,
    total_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl SpanStats {
    const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            total_ns: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
        }
    }

    fn record(&self, nanos: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_ns.fetch_add(nanos, Ordering::Relaxed);
        self.max_ns.fetch_max(nanos, Ordering::Relaxed);
    }

    fn sample(&self) -> SpanSample {
        SpanSample {
            count: self.count.load(Ordering::Relaxed),
            total_ns: self.total_ns.load(Ordering::Relaxed),
            max_ns: self.max_ns.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.total_ns.store(0, Ordering::Relaxed);
        self.max_ns.store(0, Ordering::Relaxed);
    }
}

/// Per-command invoke counts. The name table is installed once at boot (`lib.rs` passes the
/// dispatch table's command universe) and never changes afterwards, so lookup is a binary search
/// over a sorted `Vec` and the count itself is an atomic add — no map, no lock, no allocation on
/// the invoke path.
#[derive(Debug)]
struct CommandCounters {
    names: Vec<&'static str>,
    counts: Vec<AtomicU64>,
}

/// Process-wide instrumentation registry: fixed atomic slots behind a single [`AtomicBool`] gate.
///
/// **The gate is the whole design constraint.** This type is compiled into release builds too (so a
/// user can reproduce a slowdown with `TAIDE_PERF=1` instead of needing a special build), which
/// means every measurement site sits on a hot path that must cost *nothing* when instrumentation is
/// off. It costs one relaxed atomic load: [`span`] returns a [`Span`] that never called
/// `Instant::now`, and [`add`]/[`record_command`] return before touching any slot.
///
/// When the gate is on, recording stays lock-free. A `Mutex<HashMap<&str, Stat>>` — the obvious
/// first design — would have put every parallel search worker, the pty reader thread and the git
/// `spawn_blocking` pool on one lock and measured its own contention (research 3b §7).
///
/// The type is a plain value with no global state of its own; [`global`] holds the one instance the
/// app uses, and tests construct their own so they never race the process-wide one.
#[derive(Debug)]
pub struct PerfRegistry {
    enabled: AtomicBool,
    spans: [SpanStats; SPAN_SLOT_COUNT],
    counters: [AtomicU64; COUNTER_SLOT_COUNT],
    commands: OnceLock<CommandCounters>,
}

impl Default for PerfRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfRegistry {
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            spans: [const { SpanStats::new() }; SPAN_SLOT_COUNT],
            counters: [const { AtomicU64::new(0) }; COUNTER_SLOT_COUNT],
            commands: OnceLock::new(),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Installs the closed set of command names whose invocations [`PerfRegistry::record_command`]
    /// counts individually. Only the first call wins — a second one is ignored rather than
    /// resizing the count array under readers.
    pub fn install_commands(&self, names: impl IntoIterator<Item = &'static str>) {
        let mut names: Vec<&'static str> = names.into_iter().collect();
        names.sort_unstable();
        names.dedup();
        let counts = names.iter().map(|_| AtomicU64::new(0)).collect();
        let _ = self.commands.set(CommandCounters { names, counts });
    }

    pub fn record_span(&self, slot: SpanSlot, nanos: u64) {
        if !self.is_enabled() {
            return;
        }
        self.spans[slot.index()].record(nanos);
    }

    pub fn add(&self, slot: CounterSlot, amount: u64) {
        if !self.is_enabled() {
            return;
        }
        self.counters[slot.index()].fetch_add(amount, Ordering::Relaxed);
    }

    /// Counts one invocation of `name`. A name absent from the installed table (or a table that was
    /// never installed) lands in [`CounterSlot::UnlistedCommand`] instead of being dropped.
    pub fn record_command(&self, name: &str) {
        if !self.is_enabled() {
            return;
        }
        let Some(commands) = self.commands.get() else {
            self.add(CounterSlot::UnlistedCommand, 1);
            return;
        };
        match commands.names.binary_search_by(|probe| (*probe).cmp(name)) {
            Ok(index) => {
                commands.counts[index].fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => self.add(CounterSlot::UnlistedCommand, 1),
        }
    }

    pub fn span_sample(&self, slot: SpanSlot) -> SpanSample {
        self.spans[slot.index()].sample()
    }

    pub fn counter_value(&self, slot: CounterSlot) -> u64 {
        self.counters[slot.index()].load(Ordering::Relaxed)
    }

    /// Command names that were invoked at least once, in sorted name order. Zero-count commands are
    /// omitted so a snapshot stays readable — the app has well over a hundred commands and a
    /// typical session exercises a handful.
    pub fn command_samples(&self) -> Vec<(&'static str, u64)> {
        let Some(commands) = self.commands.get() else {
            return Vec::new();
        };
        commands
            .names
            .iter()
            .zip(commands.counts.iter())
            .map(|(name, count)| (*name, count.load(Ordering::Relaxed)))
            .filter(|(_, count)| *count > 0)
            .collect()
    }

    /// Zeroes every accumulated number. The gate and the installed command-name table are left
    /// alone — resetting is "start a new measurement window", not "tear the registry down".
    pub fn reset(&self) {
        for stats in &self.spans {
            stats.reset();
        }
        for counter in &self.counters {
            counter.store(0, Ordering::Relaxed);
        }
        if let Some(commands) = self.commands.get() {
            for count in &commands.counts {
                count.store(0, Ordering::Relaxed);
            }
        }
    }
}

/// RAII timer for one [`SpanSlot`]. Records on drop, so the measured region is exactly the
/// binding's lexical scope.
///
/// `started` is `None` when the registry gate was off at construction, which is what makes the
/// disabled path free: no clock read on entry, no atomic write on exit.
#[derive(Debug)]
pub struct Span<'a> {
    registry: &'a PerfRegistry,
    slot: SpanSlot,
    started: Option<Instant>,
}

impl<'a> Span<'a> {
    pub fn start(registry: &'a PerfRegistry, slot: SpanSlot) -> Self {
        Self {
            registry,
            slot,
            started: registry.is_enabled().then(Instant::now),
        }
    }
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        let Some(started) = self.started else {
            return;
        };
        let nanos = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.registry.record_span(self.slot, nanos);
    }
}

static GLOBAL: PerfRegistry = PerfRegistry::new();

/// The one registry the running app records into. Measurement sites call the free functions below
/// rather than this directly; it is public so `domain::app` can read it for `perf_snapshot`.
pub fn global() -> &'static PerfRegistry {
    &GLOBAL
}

/// Reads `TAIDE_PERF` and installs the command-name table. Call once, before the first measured
/// region — `lib.rs::run` does it at the top, ahead of `setup`.
pub fn init(command_names: impl IntoIterator<Item = &'static str>) {
    let raw = std::env::var(PERF_ENV_VAR).ok();
    GLOBAL.set_enabled(resolve_enabled(raw.as_deref(), cfg!(debug_assertions)));
    GLOBAL.install_commands(command_names);
}

/// Times the enclosing scope into `slot`. Bind it to a named local — `let _ = perf::span(..)` drops
/// immediately and measures nothing.
#[must_use = "a span records its duration when dropped, so binding it to `_` measures nothing"]
pub fn span(slot: SpanSlot) -> Span<'static> {
    Span::start(global(), slot)
}

/// Adds `amount` to `slot`. The high-frequency alternative to [`span`] — see [`CounterSlot`].
pub fn add(slot: CounterSlot, amount: u64) {
    global().add(slot, amount);
}

/// Counts one IPC invocation of `name`.
pub fn record_command(name: &str) {
    global().record_command(name);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_registry() -> PerfRegistry {
        let registry = PerfRegistry::new();
        registry.set_enabled(true);
        registry
    }

    #[test]
    fn 모든_span_슬롯은_선언_순서와_같은_인덱스를_가진다() {
        for (index, slot) in SpanSlot::ALL.iter().enumerate() {
            assert_eq!(slot.index(), index, "{} 슬롯의 인덱스가 선언 순서와 다르다", slot.name());
        }
        assert_eq!(SpanSlot::ALL.len(), SPAN_SLOT_COUNT);
    }

    #[test]
    fn 모든_counter_슬롯은_선언_순서와_같은_인덱스를_가진다() {
        for (index, slot) in CounterSlot::ALL.iter().enumerate() {
            assert_eq!(slot.index(), index, "{} 슬롯의 인덱스가 선언 순서와 다르다", slot.name());
        }
        assert_eq!(CounterSlot::ALL.len(), COUNTER_SLOT_COUNT);
    }

    #[test]
    fn 슬롯_이름은_전부_고유하고_비어있지_않다() {
        let mut names: Vec<&str> = SpanSlot::ALL.iter().map(|slot| slot.name()).collect();
        names.extend(CounterSlot::ALL.iter().map(|slot| slot.name()));
        assert!(names.iter().all(|name| !name.is_empty()));

        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "중복된 슬롯 이름이 있다");
    }

    #[test]
    fn 게이트가_꺼져_있으면_아무것도_기록하지_않는다() {
        let registry = PerfRegistry::new();
        registry.install_commands(["git_status"]);

        {
            let _span = Span::start(&registry, SpanSlot::GitStatus);
        }
        registry.record_span(SpanSlot::FileOpen, 1_000);
        registry.add(CounterSlot::PtyOutputBytes, 4_096);
        registry.record_command("git_status");

        assert_eq!(registry.span_sample(SpanSlot::GitStatus), SpanSample::default());
        assert_eq!(registry.span_sample(SpanSlot::FileOpen), SpanSample::default());
        assert_eq!(registry.counter_value(CounterSlot::PtyOutputBytes), 0);
        assert_eq!(registry.counter_value(CounterSlot::UnlistedCommand), 0);
        assert!(registry.command_samples().is_empty());
    }

    #[test]
    fn 게이트가_꺼진_span_은_드롭해도_기록하지_않는다() {
        let registry = PerfRegistry::new();
        let span = Span::start(&registry, SpanSlot::ProjectOpen);
        registry.set_enabled(true);
        drop(span);

        assert_eq!(registry.span_sample(SpanSlot::ProjectOpen), SpanSample::default());
    }

    #[test]
    fn span_기록은_횟수와_누적과_최대를_함께_올린다() {
        let registry = enabled_registry();

        registry.record_span(SpanSlot::GitStatus, 300);
        registry.record_span(SpanSlot::GitStatus, 100);
        registry.record_span(SpanSlot::GitStatus, 200);

        let sample = registry.span_sample(SpanSlot::GitStatus);
        assert_eq!(sample.count, 3);
        assert_eq!(sample.total_ns, 600);
        assert_eq!(sample.max_ns, 300, "최대값은 가장 큰 관측치를 유지해야 한다");
    }

    #[test]
    fn 슬롯끼리는_서로_간섭하지_않는다() {
        let registry = enabled_registry();

        registry.record_span(SpanSlot::FileOpen, 10);
        registry.add(CounterSlot::LspSend, 7);

        assert_eq!(registry.span_sample(SpanSlot::FileOpen).count, 1);
        assert_eq!(registry.span_sample(SpanSlot::GitStatus), SpanSample::default());
        assert_eq!(registry.counter_value(CounterSlot::LspSend), 7);
        assert_eq!(registry.counter_value(CounterSlot::PtyOutputBytes), 0);
    }

    #[test]
    fn counter_는_금액을_누적한다() {
        let registry = enabled_registry();

        registry.add(CounterSlot::PtyOutputBytes, 4_096);
        registry.add(CounterSlot::PtyOutputBytes, 512);
        registry.add(CounterSlot::PtyOutputChunks, 1);
        registry.add(CounterSlot::PtyOutputChunks, 1);

        assert_eq!(registry.counter_value(CounterSlot::PtyOutputBytes), 4_608);
        assert_eq!(registry.counter_value(CounterSlot::PtyOutputChunks), 2);
    }

    /// Long enough that the elapsed time is non-zero on any clock resolution — an empty scope can
    /// legitimately measure 0ns. This is a lower bound on the timer wiring, not a budget: no test
    /// in this file asserts an upper bound on wall time (계약 §C.2-3).
    const SPAN_TIMING_SLEEP_MS: u64 = 2;

    #[test]
    fn 실행된_span_은_경과_시간을_기록한다() {
        let registry = enabled_registry();
        {
            let _span = Span::start(&registry, SpanSlot::SearchRun);
            std::thread::sleep(std::time::Duration::from_millis(SPAN_TIMING_SLEEP_MS));
        }

        let sample = registry.span_sample(SpanSlot::SearchRun);
        assert_eq!(sample.count, 1);
        assert!(sample.max_ns > 0, "실제로 실행된 구간은 0ns 보다 커야 한다");
        assert_eq!(sample.total_ns, sample.max_ns, "1회 기록에서 누적과 최대는 같아야 한다");
    }

    #[test]
    fn 등재된_커맨드만_이름별로_집계된다() {
        let registry = enabled_registry();
        registry.install_commands(["git_status", "file_open"]);

        registry.record_command("git_status");
        registry.record_command("git_status");
        registry.record_command("file_open");

        assert_eq!(registry.command_samples(), vec![("file_open", 1), ("git_status", 2)]);
        assert_eq!(registry.counter_value(CounterSlot::UnlistedCommand), 0);
    }

    #[test]
    fn 미등재_커맨드는_unlisted_카운터로_간다() {
        let registry = enabled_registry();
        registry.install_commands(["git_status"]);

        registry.record_command("plugin:notification|is_permission_granted");
        registry.record_command("zzz_after_last_name");
        registry.record_command("aaa_before_first_name");

        assert_eq!(registry.counter_value(CounterSlot::UnlistedCommand), 3);
        assert!(registry.command_samples().is_empty());
    }

    #[test]
    fn 커맨드_표가_설치되지_않으면_전부_unlisted_다() {
        let registry = enabled_registry();

        registry.record_command("git_status");

        assert_eq!(registry.counter_value(CounterSlot::UnlistedCommand), 1);
        assert!(registry.command_samples().is_empty());
    }

    #[test]
    fn 커맨드_표는_한_번만_설치된다() {
        let registry = enabled_registry();
        registry.install_commands(["git_status"]);
        registry.install_commands(["file_open"]);

        registry.record_command("git_status");
        registry.record_command("file_open");

        assert_eq!(registry.command_samples(), vec![("git_status", 1)]);
        assert_eq!(registry.counter_value(CounterSlot::UnlistedCommand), 1);
    }

    #[test]
    fn 중복된_커맨드_이름은_한_슬롯으로_합쳐진다() {
        let registry = enabled_registry();
        registry.install_commands(["git_status", "git_status"]);

        registry.record_command("git_status");

        assert_eq!(registry.command_samples(), vec![("git_status", 1)]);
    }

    #[test]
    fn reset_은_누적치만_지우고_게이트와_커맨드_표는_유지한다() {
        let registry = enabled_registry();
        registry.install_commands(["git_status"]);
        registry.record_span(SpanSlot::GitStatus, 500);
        registry.add(CounterSlot::PtyOutputBytes, 128);
        registry.record_command("git_status");

        registry.reset();

        assert_eq!(registry.span_sample(SpanSlot::GitStatus), SpanSample::default());
        assert_eq!(registry.counter_value(CounterSlot::PtyOutputBytes), 0);
        assert!(registry.command_samples().is_empty());
        assert!(registry.is_enabled(), "reset 은 게이트를 끄지 않는다");

        registry.record_command("git_status");
        assert_eq!(
            registry.command_samples(),
            vec![("git_status", 1)],
            "reset 후에도 커맨드 표는 그대로 쓰인다"
        );
    }

    #[test]
    fn 환경변수가_없으면_빌드_기본값을_따른다() {
        assert!(resolve_enabled(None, true));
        assert!(!resolve_enabled(None, false));
    }

    #[test]
    fn 환경변수는_참값과_거짓값을_대소문자_구분_없이_해석한다() {
        for raw in ["1", "true", "TRUE", "On", " on "] {
            assert!(resolve_enabled(Some(raw), false), "{raw} 는 켜짐으로 해석돼야 한다");
        }
        for raw in ["0", "false", "FALSE", "Off", " off "] {
            assert!(!resolve_enabled(Some(raw), true), "{raw} 는 꺼짐으로 해석돼야 한다");
        }
    }

    #[test]
    fn 해석할_수_없는_환경변수_값은_빌드_기본값으로_되돌아간다() {
        for raw in ["", "  ", "yes", "2", "enabled"] {
            assert!(resolve_enabled(Some(raw), true), "{raw:?} 는 debug 기본값 on 을 따라야 한다");
            assert!(!resolve_enabled(Some(raw), false), "{raw:?} 는 release 기본값 off 를 따라야 한다");
        }
    }
}
