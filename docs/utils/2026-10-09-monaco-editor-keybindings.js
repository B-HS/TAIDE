import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { KeyChord, KeyCodeUtils } from '../../node_modules/monaco-editor/esm/vs/base/common/keyCodes.js'
import { EditorContextKeys } from '../../node_modules/monaco-editor/esm/vs/editor/common/editorContextKeys.js'
import { ContextKeyExpr } from '../../node_modules/monaco-editor/esm/vs/platform/contextkey/common/contextkey.js'

const ROOT = resolve(import.meta.dir, '../..')
const SOURCE = resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib')
const KEY_CODE_MASK = 255
const CTRL_CMD = 2048
const WIN_CTRL = 256
const SHIFT = 1024
const ALT = 512
const CHORD_MASK = 65535
const CHORD_SHIFT = 16
const modules = [
    'linesOperations/browser/linesOperations.js',
    'comment/browser/comment.js',
    'caretOperations/browser/transpose.js',
    'caretOperations/browser/caretOperations.js',
    'bracketMatching/browser/bracketMatching.js',
    'multicursor/browser/multicursor.js',
    'smartSelect/browser/smartSelect.js',
    'cursorUndo/browser/cursorUndo.js',
    'lineSelection/browser/lineSelection.js',
    'anchorSelect/browser/anchorSelect.js',
]
const keys = { up: 'ArrowUp', down: 'ArrowDown', left: 'ArrowLeft', right: 'ArrowRight' }
const stage = (value) => ({
    key: keys[KeyCodeUtils.toUserSettingsUS(value & KEY_CODE_MASK).toLowerCase()] ?? KeyCodeUtils.toUserSettingsUS(value & KEY_CODE_MASK),
    mods: [
        [CTRL_CMD, 'mod'],
        [WIN_CTRL, 'ctrl'],
        [SHIFT, 'shift'],
        [ALT, 'alt'],
    ]
        .filter(([flag]) => (value & flag) !== 0)
        .map(([, name]) => name),
})
let definitions = []
const EditorAction = class {
    constructor(options) {
        this.options = options
    }
}
const registerEditorAction = (Action) => {
    definitions = [...definitions, new Action().options]
}
const localize = (_, text) => text
const localize2 = localize
const MenuId = {}
const CommandsRegistry = { registerCommandAlias: () => {} }
const selectClasses = (original, first, end, names) =>
    `${original.slice(original.indexOf(first), original.indexOf(end))}\n${names.map((name) => `registerEditorAction(${name})`).join('\n')}`
for (const path of modules) {
    const original = readFileSync(resolve(SOURCE, path), 'utf8')
    let selected = original
    if (path.startsWith('bracketMatching/'))
        selected = selectClasses(original, 'class JumpToBracketAction', 'class BracketsData', [
            'JumpToBracketAction',
            'SelectToBracketAction',
            'RemoveBracketsAction',
        ])
    if (path.startsWith('multicursor/'))
        selected = [
            selectClasses(original, 'class InsertCursorAbove', 'class MultiCursorSessionResult', [
                'InsertCursorAbove',
                'InsertCursorBelow',
                'InsertCursorAtEndOfEachLineSelected',
                'InsertCursorAtEndOfLineSelected',
                'InsertCursorAtTopOfLineSelected',
            ]),
            selectClasses(original, 'class MultiCursorSelectionControllerAction', 'class SelectionHighlighterState', [
                'AddSelectionToNextFindMatchAction',
                'AddSelectionToPreviousFindMatchAction',
                'MoveSelectionToNextFindMatchAction',
                'MoveSelectionToPreviousFindMatchAction',
                'SelectHighlightsAction',
                'CompatChangeAll',
            ]),
            selectClasses(original, 'class FocusNextCursor', 'registerEditorContribution(MultiCursorSelectionController', [
                'FocusNextCursor',
                'FocusPreviousCursor',
            ]),
        ].join('\n')
    if (path.startsWith('smartSelect/'))
        selected = selectClasses(original, 'class AbstractSmartSelect', 'registerEditorContribution(SmartSelectController', [
            'GrowSelectionAction',
            'ShrinkSelectionAction',
        ])
    if (path.startsWith('cursorUndo/'))
        selected = selectClasses(original, 'class CursorUndo extends', 'registerEditorContribution(CursorUndoRedoController', [
            'CursorUndo',
            'CursorRedo',
        ])
    if (path.startsWith('anchorSelect/'))
        selected = selectClasses(original, 'class SetSelectionAnchor', 'class GoToSelectionAnchor', ['SetSelectionAnchor'])
    const source = selected.replace(/^import .*;$/gm, '').replace(/^export .*;$/gm, '')
    eval(source)
}
const result = definitions.flatMap((definition) =>
    ['mac', 'win', 'linux'].flatMap((platform) => {
        const binding = definition.kbOpts
        if (!binding) return []
        const selected = binding[platform] ?? binding
        return [selected.primary ?? binding.primary, ...(selected.secondary ?? binding.secondary ?? [])].filter(Boolean).map((value) => ({
            id: `monaco.${definition.id}`,
            platform,
            ...stage(value & CHORD_MASK),
            ...(value >>> CHORD_SHIFT ? { chord: stage(value >>> CHORD_SHIFT) } : {}),
        }))
    }),
)
writeFileSync(resolve(ROOT, 'native/taide-native-ui/src/editor-keymap-defaults.json'), `${JSON.stringify(result, null, 4)}\n`)
process.exit(0)
