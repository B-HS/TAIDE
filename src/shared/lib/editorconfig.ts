import type { EditorConfigOptions } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

const INSERT_SPACES_BY_INDENT_STYLE = { tab: false, space: true } as const

/**
 * The tab width a file's `.editorconfig` asks for, or `null` when it asks for none.
 *
 * Which of the two size properties wins depends on `indent_style`, because they mean different
 * things: with tabs the on-screen width of the indent character *is* `tab_width`, while with spaces
 * the indent is `indent_size` columns wide and `tab_width` only describes a tab that happens to be
 * in the file. Monaco has a single `tabSize` for both roles, so each style picks the property that
 * governs it and falls back to the other one when it was left out — which is also how
 * `tab_width`'s documented "defaults to indent_size" is honored without materializing it in Rust.
 */
const resolveEditorConfigTabSize = (editorConfig: EditorConfigOptions) =>
    editorConfig.indentStyle === 'tab' ? (editorConfig.tabWidth ?? editorConfig.indentSize) : (editorConfig.indentSize ?? editorConfig.tabWidth)

/**
 * Splits a file's resolved `.editorconfig` into the two `CodeEditor` props that carry an indent
 * override, both `null` when the config says nothing about indentation (no `.editorconfig`, the
 * `editorConfigEnabled` setting off, or a config that only sets non-indent properties). Kept as two
 * primitives rather than one object so `CodeEditor`'s effect can depend on them directly — an object
 * rebuilt each render would re-run that effect on every keystroke.
 */
export const resolveEditorConfigIndentProps = (editorConfig: EditorConfigOptions | null | undefined) => ({
    editorConfigTabSize: editorConfig ? resolveEditorConfigTabSize(editorConfig) : null,
    editorConfigInsertSpaces: editorConfig?.indentStyle ? INSERT_SPACES_BY_INDENT_STYLE[editorConfig.indentStyle] : null,
})

type EditorConfigIndentProps = ReturnType<typeof resolveEditorConfigIndentProps>

/**
 * The `ITextModel.updateOptions` payload for an `.editorconfig` indent override, or `null` when
 * there is nothing to override — in which case the caller must leave the model alone entirely, so
 * that a file with no `.editorconfig` keeps exactly the indentation monaco's own settings/detection
 * path gave it.
 *
 * An axis the config left unspecified is omitted rather than filled in: `updateOptions` treats an
 * absent (or `undefined`) field as "keep what the model has", so a config that only says
 * `indent_style = tab` flips `insertSpaces` and leaves the detected/configured tab width standing.
 */
export const resolveEditorConfigModelIndent = ({
    editorConfigTabSize,
    editorConfigInsertSpaces,
}: EditorConfigIndentProps): monaco.editor.ITextModelUpdateOptions | null => {
    if (editorConfigTabSize === null && editorConfigInsertSpaces === null) return null
    return { tabSize: editorConfigTabSize ?? undefined, insertSpaces: editorConfigInsertSpaces ?? undefined }
}

type OnSaveCleanupFlags = {
    trimTrailingWhitespaceOnSave: boolean | undefined
    insertFinalNewlineOnSave: boolean | undefined
}

/**
 * Lets a file's `.editorconfig` decide the two on-save cleanups (d-53 U2) for that file, falling
 * back to the global settings for whichever property it does not mention. An explicit `false` in the
 * config wins over a global `true` — that is the point of a per-file override, and `??` (rather than
 * `||`) is what keeps it from being read as "unset".
 */
export const resolveOnSaveCleanupFlags = (
    settings: OnSaveCleanupFlags,
    editorConfig: EditorConfigOptions | null | undefined,
): OnSaveCleanupFlags => ({
    trimTrailingWhitespaceOnSave: editorConfig?.trimTrailingWhitespace ?? settings.trimTrailingWhitespaceOnSave,
    insertFinalNewlineOnSave: editorConfig?.insertFinalNewline ?? settings.insertFinalNewlineOnSave,
})
