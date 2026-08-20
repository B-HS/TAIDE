use std::time::{SystemTime, UNIX_EPOCH};

pub const MS_PER_SECOND: f64 = 1_000.0;

/// Epoch milliseconds "now", for every `f64` epoch-ms field the IPC time-field convention covers
/// (`docs/data-model.md` §6) — `Project.last_opened_at`, `MirrorFile.saved_at_ms`, and any future
/// one. Falls back to `0.0` on a clock error (pre-1970 system clock) rather than panicking. Lives
/// in `infra` (not a domain module) because `domain::project::service` and `domain::file::service`
/// both need the exact same clock read and domains may not call into each other — this is the
/// shared, domain-agnostic home the same descent `infra::archive::extract_hardened_zip` and
/// `infra::redact::mask_provider_error` already took for cross-domain utilities.
pub fn now_epoch_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64() * MS_PER_SECOND)
        .unwrap_or(0.0)
}
