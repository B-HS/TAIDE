import type { monaco } from '@shared/lib/monaco/setup'
import { INSERT_FINAL_NEW_LINE_ACTION_ID, TRIM_TRAILING_WHITESPACE_ACTION_ID } from '@shared/lib/monaco/monaco-actions'

/**
 * What monaco's `editor.action.trimTrailingWhitespace` reads to decide whether the caret's own line
 * is exempt from the trim: with this argument it collects the current cursor positions and leaves
 * the whitespace to their left alone, without it it trims every line. Passed only for auto-save,
 * matching VS Code's own save participant (`isAutoSaved`) — a timer firing mid-typing must not eat
 * the indentation the user is in the middle of writing, while an explicit ⌘S is a deliberate "clean
 * this file up now".
 */
const AUTO_SAVE_TRIM_ARGS = { reason: 'auto-save' } as const

type OnSaveCleanupEditor = Pick<monaco.editor.IStandaloneCodeEditor, 'getAction'>

/**
 * Runs one cleanup action and swallows whatever it does on the way out, so a step that cannot run
 * never cancels the save it was decorating.
 *
 * The `try`/`catch` is load-bearing rather than stylistic: `IEditorAction.run` is only nominally
 * async. Monaco's `InternalEditorAction.run` hands straight to the widget's runner, which is
 * `instantiationService.invokeFunction(...)` — an ordinary synchronous call — so anything the
 * action throws before it reaches its own `Promise.resolve` (a failing `executeCommands`, a
 * telemetry/context-key access) comes back out of `run()` as a synchronous throw. A `.catch()`
 * chained onto the call's result is never even evaluated in that case, which used to turn a failed
 * cleanup into a rejected `handleSave` — no `file_save` at all, `savingRef` left standing.
 */
const runCleanupAction = async (editor: OnSaveCleanupEditor, actionId: string, args?: typeof AUTO_SAVE_TRIM_ARGS) => {
    try {
        await editor.getAction(actionId)?.run(args)
    } catch {
        return undefined
    }
}

type RunOnSaveCleanupInput = {
    editor: OnSaveCleanupEditor | null
    trimTrailingWhitespaceOnSave: boolean | undefined
    insertFinalNewlineOnSave: boolean | undefined
    isAutoSave: boolean
}

/**
 * Runs the enabled on-save text cleanups against the buffer that is about to be written, in the
 * order VS Code applies them: trailing whitespace first, then the final newline — a whitespace-only
 * last line has to become empty before the newline step looks at it, or the file gains a newline
 * behind whitespace that was itself about to be trimmed.
 *
 * Both steps are monaco's own built-in actions (the same ones the keymap exposes as manually
 * invokable rows, `monaco-actions.ts`), not a second trimming implementation: they push their edits
 * through `executeCommands`, whose command objects track the live selection across the edit, so the
 * caret and any multi-cursor selections survive the cleanup instead of being reset to the top of the
 * document the way a `setValue` would.
 *
 * The caller runs this *after* format-on-save so the formatter's own output is cleaned too, and
 * before reading the draft it sends to disk — the edits land on the model, so the pane's usual
 * `onDidChangeContent` → `onChange` path picks them up as the current draft. Failures are swallowed
 * per step ({@link runCleanupAction}) for the same reason the format step swallows its own: a
 * cleanup that cannot run must not cancel the save it was decorating.
 */
export const runOnSaveCleanup = async ({ editor, trimTrailingWhitespaceOnSave, insertFinalNewlineOnSave, isAutoSave }: RunOnSaveCleanupInput) => {
    if (!editor) return

    if (trimTrailingWhitespaceOnSave) {
        await runCleanupAction(editor, TRIM_TRAILING_WHITESPACE_ACTION_ID, isAutoSave ? AUTO_SAVE_TRIM_ARGS : undefined)
    }

    if (insertFinalNewlineOnSave) await runCleanupAction(editor, INSERT_FINAL_NEW_LINE_ACTION_ID)
}
