import type { SnippetEntry, SnippetFile, SnippetStringOrList } from '@shared/api/bindings'
import {
    isGlobalSnippetFileName,
    SNIPPET_GLOBAL_FILE_EXTENSION,
    SNIPPET_LANGUAGE_FILE_EXTENSION,
    stringOrListToLines,
} from '@shared/lib/snippet-file'

const SNIPPET_CONTENT_INDENT_SPACES = 4

export type SnippetEntryDraft = {
    id: string
    name: string
    prefix: string
    body: string
    description: string
    scope: string
}

export const buildLanguageSnippetFileName = (languageId: string) => `${languageId}${SNIPPET_LANGUAGE_FILE_EXTENSION}`

export const normalizeGlobalSnippetFileName = (rawName: string) => {
    const trimmed = rawName.trim()
    return isGlobalSnippetFileName(trimmed) ? trimmed : `${trimmed}${SNIPPET_GLOBAL_FILE_EXTENSION}`
}

/**
 * Mirrors the Rust-side `has_unsafe_path_characters` (`domain/snippet/service.rs`) so a name the
 * new-file dialog lets through never bounces back as a save failure — `/`, `\`, `..`, and `:` (the
 * Windows drive-relative-path escape, `docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md`
 * §5's finding) are all rejected client-side before the IPC round trip, not just server-side.
 */
export const isSafeSnippetFileName = (fileName: string) => !/[/\\:]/.test(fileName) && !fileName.includes('..')

const stringOrListToInline = (value: SnippetStringOrList | null | undefined) => (Array.isArray(value) ? value.join(', ') : (value ?? ''))

/** Multiple lines become a `string[]` body (readable multi-line JSON, matching VS Code's own snippet files); a single line stays a plain string. */
const linesToStringOrList = (text: string) => {
    const lines = text.split('\n')
    return lines.length > 1 ? lines : (lines[0] ?? '')
}

/** Multiple comma-separated values become a `string[]` (`prefix`'s multiple triggers, VS Code's own array shape for it); one value stays a plain string. Only `prefix` uses comma semantics — `description` is free prose and is never split (see `draftsToSnippetContent`). */
const inlineToStringOrList = (text: string) => {
    const parts = text
        .split(',')
        .map((part) => part.trim())
        .filter((part) => part.length > 0)
    return parts.length > 1 ? parts : (parts[0] ?? text.trim())
}

export const snippetMapToDrafts = (snippets: SnippetFile['snippets']): SnippetEntryDraft[] =>
    (Object.entries(snippets) as [string, SnippetEntry][]).map(([name, entry]) => ({
        id: crypto.randomUUID(),
        name,
        prefix: stringOrListToInline(entry.prefix),
        body: stringOrListToLines(entry.body),
        description: entry.description ? stringOrListToLines(entry.description) : '',
        scope: entry.scope ?? '',
    }))

export const createEmptySnippetEntryDraft = (): SnippetEntryDraft => ({
    id: crypto.randomUUID(),
    name: '',
    prefix: '',
    body: '',
    description: '',
    scope: '',
})

export const appendSnippetEntryDraft = (drafts: SnippetEntryDraft[]): SnippetEntryDraft[] => [...drafts, createEmptySnippetEntryDraft()]

export const updateSnippetEntryDraft = (drafts: SnippetEntryDraft[], id: string, patch: Partial<SnippetEntryDraft>): SnippetEntryDraft[] =>
    drafts.map((draft) => (draft.id === id ? { ...draft, ...patch } : draft))

export const removeSnippetEntryDraft = (drafts: SnippetEntryDraft[], id: string): SnippetEntryDraft[] => drafts.filter((draft) => draft.id !== id)

export const isSnippetEntryDraftValid = (draft: SnippetEntryDraft) =>
    draft.name.trim().length > 0 && draft.prefix.trim().length > 0 && draft.body.trim().length > 0

const isSnippetEntryDraftBlank = (draft: SnippetEntryDraft) =>
    [draft.name, draft.prefix, draft.body, draft.description, draft.scope].every((field) => field.trim().length === 0)

/**
 * Whether any draft carries typed content but is missing one of the three fields
 * {@link draftsToSnippetContent} requires. That serializer *drops* such entries, so saving reported
 * success while the half-filled snippet vanished — the editor still showed its row until the file
 * was reselected, at which point the typing was simply gone (audit §4-B D6). A completely blank row
 * (added with "+" and never filled in) is excluded on purpose: it carries nothing to lose, so
 * blocking the save on it would only make an abandoned row impossible to save around.
 */
export const findIncompleteSnippetEntryDrafts = (drafts: SnippetEntryDraft[]) =>
    drafts.filter((draft) => !isSnippetEntryDraftBlank(draft) && !isSnippetEntryDraftValid(draft))

const serializeSnippetDraftFields = (drafts: SnippetEntryDraft[]) =>
    JSON.stringify(drafts.map(({ name, prefix, body, description, scope }) => [name, prefix, body, description, scope]))

/**
 * Whether the editor's drafts still match the saved file. Compares the drafts *as edited* — not
 * {@link draftsToSnippetContent} output — so a half-filled entry (which that serializer drops) still
 * counts as an unsaved change; the editor discarded exactly those on a file switch or on leaving,
 * with no prompt (audit §4-B D6). Draft ids are excluded because they are regenerated on every load.
 */
export const hasUnsavedSnippetDraftChanges = (drafts: SnippetEntryDraft[], savedSnippets: SnippetFile['snippets']) =>
    serializeSnippetDraftFields(drafts) !== serializeSnippetDraftFields(snippetMapToDrafts(savedSnippets))

/**
 * Whether two or more otherwise-valid drafts (per {@link isSnippetEntryDraftValid}) share the same
 * trimmed `name` — the key `draftsToSnippetContent` below serializes them under. `Object.fromEntries`
 * silently keeps only the last entry for a repeated key, so without this check a duplicate name
 * would drop an earlier entry's `prefix`/`body` with no error and no visible sign in the editor
 * (the still-duplicate-named draft rows remain on screen after save).
 */
export const hasDuplicateSnippetEntryNames = (drafts: SnippetEntryDraft[]) => {
    const names = drafts.filter(isSnippetEntryDraftValid).map((draft) => draft.name.trim())
    return new Set(names).size !== names.length
}

/**
 * Assembles the full file's content JSON string `snippet_save` persists verbatim — entries with an
 * empty name/prefix/body are dropped. `description` is always written as the plain string the
 * single-line editor field holds (never split on commas like `prefix`) — VS Code's own `string[]`
 * shape for this field means multiple *lines*, not a comma-separated list, and the editor UI here
 * has no multi-line input for it to legitimately produce that shape.
 */
export const draftsToSnippetContent = (drafts: SnippetEntryDraft[]) => {
    const entries = drafts.filter(isSnippetEntryDraftValid).map((draft): [string, SnippetEntry] => [
        draft.name.trim(),
        {
            prefix: inlineToStringOrList(draft.prefix),
            body: linesToStringOrList(draft.body),
            ...(draft.description.trim() ? { description: draft.description.trim() } : {}),
            ...(draft.scope.trim() ? { scope: draft.scope.trim() } : {}),
        },
    ])
    return JSON.stringify(Object.fromEntries(entries), null, SNIPPET_CONTENT_INDENT_SPACES)
}
