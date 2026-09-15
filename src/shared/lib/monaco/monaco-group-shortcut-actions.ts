import type { EditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import type { KeymapActionId, KeymapEntry } from '@shared/lib/keymap/keymap'
import { buildMonacoChordKeybinding } from '@shared/lib/monaco/monaco-keybinding'

/**
 * The seven ⌘K chords the chord engine deliberately never arms while the editor holds text focus:
 * a chord's first stage carries an implicit `!editorTextFocus` gate so monaco's own ⌘K namespace
 * stays reachable (Wave H contract §2.5), which left these group/tab shortcuts dead in the one
 * state they are most used from — mid-edit (d-59 review G-1). Mirroring them as monaco actions
 * gives the editor-focused case its own path: monaco resolves ⌘K itself and runs the action, which
 * publishes the exact same `EditorPaneCommand` the app-level handler would have.
 *
 * ⌘1..⌘9 are absent on purpose — they are single-stage entries, and only *chords* are gated on
 * editor focus, so they already fire from inside the editor through the normal dispatch path.
 */
const EDITOR_GROUP_SHORTCUT_COMMANDS: Partial<Record<KeymapActionId, EditorPaneCommand>> = {
    'focus-group-left': { type: 'focus-group', target: { kind: 'direction', direction: 'left' } },
    'focus-group-right': { type: 'focus-group', target: { kind: 'direction', direction: 'right' } },
    'focus-group-up': { type: 'focus-group', target: { kind: 'direction', direction: 'up' } },
    'focus-group-down': { type: 'focus-group', target: { kind: 'direction', direction: 'down' } },
    'move-tab-to-group-left': { type: 'move-tab-to-group', direction: 'left' },
    'move-tab-to-group-right': { type: 'move-tab-to-group', direction: 'right' },
    'close-all-tabs': { type: 'close-all-tabs' },
}

/** Whether a keymap entry is one of the seven mirrored into monaco — the membership test `decideKeymapDispatch`'s regression suite and the catalog checks share. */
export const isEditorGroupShortcutKeymapId = (keymapId: KeymapActionId) => keymapId in EDITOR_GROUP_SHORTCUT_COMMANDS

/**
 * Action ids are the mirrored keymap id verbatim rather than the camelCase spelling
 * `TAIDE_CUSTOM_ACTIONS` uses, because these are not catalog rows: they are never registered as
 * global monaco commands, never listed in `MONACO_ACTIONS`, and so never become a second,
 * separately rebindable keybindings-editor row competing with the `APP_KEYMAP` entry they mirror.
 * Keeping the keymap id in the id is what makes that mirroring visible wherever monaco surfaces the
 * action (F1, `getSupportedActions()`).
 */
export const EDITOR_GROUP_SHORTCUT_ACTION_ID_PREFIX = 'taide.'

export type EditorGroupShortcutAction = {
    keymapId: KeymapActionId
    actionId: string
    labelKey: string
    keybindings: number[]
    command: EditorPaneCommand
}

/**
 * Turns the *effective* keymap (`APP_KEYMAP` with the user's overrides already applied) into the
 * monaco action descriptors to register, so a rebound chord reaches monaco under the user's own
 * keys rather than the shipped defaults.
 *
 * `keybindings` is empty when the entry no longer carries a `chord` — rebinding one of these to a
 * plain single-stage key removes the reason this mirror exists at all (only chords are gated on
 * editor focus), and the app-level dispatch already wins that keydown through window capture, so
 * registering a monaco keybinding for it would be a second owner of the same press. It is also
 * empty when a stage's key has no monaco KeyCode; the action itself is still registered in both
 * cases, keeping it runnable from monaco's own command palette.
 */
export const buildEditorGroupShortcutActions = (entries: KeymapEntry[]): EditorGroupShortcutAction[] =>
    entries.flatMap((entry) => {
        const command = EDITOR_GROUP_SHORTCUT_COMMANDS[entry.id]
        if (!command) return []
        const keybinding = entry.chord ? buildMonacoChordKeybinding(entry, entry.chord) : null
        return [
            {
                keymapId: entry.id,
                actionId: `${EDITOR_GROUP_SHORTCUT_ACTION_ID_PREFIX}${entry.id}`,
                labelKey: entry.descriptionKey,
                keybindings: keybinding === null ? [] : [keybinding],
                command,
            },
        ]
    })
