use log::{Level, LevelFilter, Log, Metadata, Record};

const LEVEL: Level = Level::Warn;

/// Writes warnings and errors to standard error so a failed launch shows its cause.
pub struct Logger;

static LOGGER: Logger = Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= LEVEL
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", line(record));
        }
    }

    fn flush(&self) {}
}

fn line(record: &Record<'_>) -> String {
    format!(
        "[{}] {}: {}",
        record.level(),
        record.target(),
        record.args()
    )
}

/// Installs [`Logger`] unless another logger was installed first.
pub fn install() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Warn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 경고_이상만_기록하고_수준과_대상과_문구를_한_줄로_쓴다() {
        let metadata = |level| Metadata::builder().level(level).build();
        assert!(Logger.enabled(&metadata(Level::Error)));
        assert!(Logger.enabled(&metadata(Level::Warn)));
        assert!(!Logger.enabled(&metadata(Level::Info)));
        assert!(!Logger.enabled(&metadata(Level::Debug)));
        assert_eq!(
            line(
                &Record::builder()
                    .level(Level::Warn)
                    .target("taide")
                    .args(format_args!("launch failed"))
                    .build()
            ),
            "[WARN] taide: launch failed"
        );
    }
}
