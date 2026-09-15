/**
 * Hard cap on how many matches one search run reports back, mirroring
 * `src-tauri/src/domain/search/types.rs`'s `SEARCH_MATCH_LIMIT`. That constant is a plain Rust
 * `const`, not part of the specta-generated IPC surface, so the value has to be restated here; the
 * Rust side stays the single source of truth and this must move with it.
 *
 * A run whose reported total reaches this stopped because the shared match budget ran out, not
 * because the project ran out of matches — the results are truncated and the UI has to say so
 * (audit §4-B C10), otherwise "10,000 results" reads as a complete answer.
 */
export const SEARCH_MATCH_LIMIT = 10_000

/**
 * Whether the search panel runs as the user types, and how long typing must pause first — both
 * mirroring `src-tauri/src/domain/settings/types.rs` (`Settings::search_on_type`,
 * `DEFAULT_SEARCH_ON_TYPE_DEBOUNCE_MS`) and the `[50, 2000]` clamp `settings::service::sanitize`
 * applies. The bounds are restated here because they are plain Rust `const`s outside the
 * specta-generated IPC surface, and they double as the settings field's `min`/`max`: a value the
 * field accepts but the backend would clamp would silently snap back after the write.
 */
export const DEFAULT_SEARCH_ON_TYPE = true

export const DEFAULT_SEARCH_ON_TYPE_DEBOUNCE_MS = 300

export const MIN_SEARCH_ON_TYPE_DEBOUNCE_MS = 50

export const MAX_SEARCH_ON_TYPE_DEBOUNCE_MS = 2_000
