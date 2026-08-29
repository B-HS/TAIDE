/**
 * Mirrors the backend's `EDITOR_RULER_COLUMN_MIN`/`EDITOR_RULER_COLUMN_MAX`/`EDITOR_RULERS_MAX`
 * (`src-tauri/src/domain/settings/service.rs`) so a column typed into the settings field is never
 * silently dropped by `sanitize_editor_rulers` after it was accepted here.
 */
export const EDITOR_RULER_COLUMN_MIN = 1
export const EDITOR_RULER_COLUMN_MAX = 1_000
export const EDITOR_RULERS_MAX = 16

/**
 * Stable empty list for "no rulers configured" — Monaco's `rulers` option is compared by the
 * option-sync effect's dependency array, so a fresh `[]` per render would re-run `updateOptions`
 * on every render of every editor host. `readonly` because that one array is shared by every editor
 * host in the app: a consumer that pushed into it would be handing its own columns to all of them.
 */
export const NO_EDITOR_RULERS: readonly number[] = []

/**
 * Parses the settings screen's comma-separated ruler columns into the `number[]` the
 * `editorRulers` setting stores, applying the same normalization the backend's
 * `sanitize_editor_rulers` applies (out-of-range dropped, deduplicated, ascending, capped) so the
 * value shown after a save is the value that was sent.
 */
export const parseEditorRulers = (value: string) => {
    const columns = value
        .split(',')
        .map((part) => part.trim())
        .filter((part) => part.length > 0)
        .map(Number)
        .filter((column) => Number.isInteger(column) && column >= EDITOR_RULER_COLUMN_MIN && column <= EDITOR_RULER_COLUMN_MAX)
    return [...new Set(columns)].sort((left, right) => left - right).slice(0, EDITOR_RULERS_MAX)
}

export const formatEditorRulers = (rulers: readonly number[]) => rulers.join(', ')

/**
 * The text the settings field must show once a value has been committed: what
 * {@link formatEditorRulers} would draw for the list {@link parseEditorRulers} actually stores.
 *
 * The field writes this back on blur rather than waiting for the stored value to come back and
 * remount it, because a normalization that resolves to the list already stored (typing `80, 2000`
 * over a stored `[80]`, or `abc` over an empty list) never changes the stored value at all — with
 * nothing to remount on, the discarded text would stay on screen as if it were in effect.
 */
export const normalizeEditorRulersText = (value: string) => formatEditorRulers(parseEditorRulers(value))
