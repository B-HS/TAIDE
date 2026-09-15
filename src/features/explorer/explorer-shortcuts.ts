import { IS_MAC } from '@shared/constants/platform'
import type { KeymapChordStage, KeymapEvent } from '@shared/lib/keymap/keymap'
import { matchesKeymapEntry } from '@shared/lib/keymap/keymap'

export type ExplorerShortcutId =
    | 'rename'
    | 'openPinned'
    | 'preview'
    | 'delete'
    | 'cut'
    | 'copy'
    | 'paste'
    | 'newFile'
    | 'newFolder'
    | 'revealInFinder'
    | 'copyPath'
    | 'copyRelativePath'

type ExplorerShortcutBinding = KeymapChordStage & { id: ExplorerShortcutId }

/**
 * The file tree's own keys. Deliberately *not* `APP_KEYMAP` entries: they only mean anything while
 * the tree owns focus, they are resolved by the tree's local `onKeyDown` rather than the window
 * capture listener, and they are therefore not rebindable from the keybindings editor
 * (`docs/features/explorer-sidebar.md` §2.5).
 *
 * Every binding here must stay clear of the `when`-less `APP_KEYMAP` entries, which win in the
 * capture phase and never reach this handler — Cmd+Down is the one overlap and is safe because the
 * terminal entry that owns it is gated on `terminalFocus`.
 *
 * `rename` is listed twice on purpose: Enter is the macOS binding and F2 the cross-platform one,
 * exactly as VS Code registers both for `renameFile`.
 */
const EXPLORER_SHORTCUT_BINDINGS: ExplorerShortcutBinding[] = [
    { id: 'rename', key: 'Enter', mods: [] },
    { id: 'rename', key: 'F2', mods: [] },
    { id: 'openPinned', key: 'ArrowDown', mods: ['mod'] },
    { id: 'preview', key: ' ', mods: [] },
    { id: 'delete', key: 'Backspace', mods: ['mod'] },
    { id: 'cut', key: 'x', mods: ['mod'] },
    { id: 'copy', key: 'c', mods: ['mod'] },
    { id: 'paste', key: 'v', mods: ['mod'] },
    { id: 'newFile', key: 'n', mods: ['mod'] },
    { id: 'newFolder', key: 'n', mods: ['mod', 'shift'] },
    { id: 'revealInFinder', key: 'r', mods: ['mod', 'alt'] },
    { id: 'copyPath', key: 'c', mods: ['mod', 'alt'] },
    { id: 'copyRelativePath', key: 'c', mods: ['mod', 'alt', 'shift'] },
]

/**
 * Display labels for the context menu. Written out rather than derived because the keys these
 * bindings use have no printable form `formatKeymapShortcut` could produce (it upper-cases any raw
 * key it has no glyph for, giving "BACKSPACE"/"ENTER"), so the glyphs are spelled here and
 * `explorer-shortcuts.test.ts` pins every label's modifier prefix to `formatKeymapShortcut` so the
 * two tables cannot drift apart. Modifier order follows that helper's macOS order (Control, Option,
 * Shift, Command), which is why relative-path copy reads "⌥⇧⌘C".
 */
export const EXPLORER_SHORTCUT_LABELS: Record<ExplorerShortcutId, string> = {
    rename: '↩',
    openPinned: '⌘↓',
    preview: '␣',
    delete: '⌘⌫',
    cut: '⌘X',
    copy: '⌘C',
    paste: '⌘V',
    newFile: '⌘N',
    newFolder: '⇧⌘N',
    revealInFinder: '⌥⌘R',
    copyPath: '⌥⌘C',
    copyRelativePath: '⌥⇧⌘C',
}

/** The bindings a label consistency check reads; not part of the runtime contract. */
export const explorerShortcutBindingsOf = (id: ExplorerShortcutId) => EXPLORER_SHORTCUT_BINDINGS.filter((binding) => binding.id === id)

/**
 * Resolves a keydown to the explorer action it triggers, or `null` when the tree does not own that
 * key. Modifier matching is exact (`matchesKeymapEntry`), so Cmd+C and Option+Cmd+C never shadow
 * each other and the list order carries no meaning.
 */
export const findExplorerShortcutId = (event: KeymapEvent, isMac: boolean = IS_MAC) =>
    EXPLORER_SHORTCUT_BINDINGS.find((binding) => matchesKeymapEntry(binding, event, isMac))?.id ?? null
