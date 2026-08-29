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
