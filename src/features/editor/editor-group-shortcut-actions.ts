import type { TFunction } from 'i18next'
import { requestEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { APP_KEYMAP, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap/keymap'
import { buildEditorGroupShortcutActions } from '@shared/lib/monaco/monaco-group-shortcut-actions'
import type { monaco } from '@shared/lib/monaco/setup'

/**
 * Registers the editor-group ⌘K chords (`monaco-group-shortcut-actions.ts`) on `editorInstance` and
 * returns a disposable that unregisters all of them — the same `editor.addAction` shape
 * `attachAiInlineEditAction` uses, and the same reason for existing as a module rather than a block
 * inside `CodeEditor`.
 *
 * `keymapOverridesJson` is the raw `settings.keymapOverrides` string rather than parsed entries so
 * the caller's effect can depend on a stable primitive: the effective keymap is a fresh array every
 * render, which as a dependency would tear down and re-register all seven actions on each one.
 *
 * Scoped with `keybindingContext: 'editorTextFocus'` (not `precondition`) for the reason spelled out
 * in `attachAiInlineEditAction`: `precondition` also gates `run()`, which would make the action a
 * silent no-op when monaco's own command palette — not the editor — holds focus.
 */
export const attachEditorGroupShortcutActions = (
    editorInstance: monaco.editor.IStandaloneCodeEditor,
    keymapOverridesJson: string | null,
    t: TFunction,
): monaco.IDisposable => {
    const entries = applyKeymapOverrides(APP_KEYMAP, parseKeymapOverrides(keymapOverridesJson))
    const actions = buildEditorGroupShortcutActions(entries).map((descriptor) =>
        editorInstance.addAction({
            id: descriptor.actionId,
            label: t(descriptor.labelKey),
            keybindingContext: 'editorTextFocus',
            keybindings: descriptor.keybindings,
            run: () => requestEditorPaneCommand(descriptor.command),
        }),
    )

    return { dispose: () => actions.forEach((action) => action.dispose()) }
}
