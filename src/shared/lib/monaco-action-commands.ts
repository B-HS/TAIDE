import type { AppCommand } from '@shared/lib/command-registry'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { MONACO_ACTIONS } from '@shared/lib/monaco-actions'
import { MONACO_ACTION_ID_PREFIX } from '@shared/lib/monaco-keybinding'

/**
 * `registerAction2` ids among `MONACO_ACTIONS` (TSV `kind` column) — these never appear in
 * `editor.getSupportedActions()`, so they cannot be gated by `CommandContext.activeEditorActionIds`
 * and stay always-enabled (a no-op `trigger()` on an unsupported editor is harmless).
 */
const MONACO_ACTION2_IDS = new Set([
    'editor.action.goToImplementation',
    'editor.action.goToLocation',
    'editor.action.goToReferences',
    'editor.action.goToTypeDefinition',
    'editor.action.inlineSuggest.toggleAlwaysShowToolbar',
    'editor.action.peekDeclaration',
    'editor.action.peekDefinition',
    'editor.action.peekImplementation',
    'editor.action.peekTypeDefinition',
    'editor.action.quickFix',
    'editor.action.referenceSearch.trigger',
    'editor.action.revealDeclaration',
    'editor.action.revealDefinition',
    'editor.action.revealDefinitionAside',
    'editor.action.showOrFocusStandaloneColorPicker',
    'editor.action.toggleStickyScroll',
    'editor.action.toggleTabFocusMode',
    'editor.action.unicodeHighlight.disableHighlightingOfAmbiguousCharacters',
    'editor.action.unicodeHighlight.disableHighlightingOfInvisibleCharacters',
    'editor.action.unicodeHighlight.disableHighlightingOfNonBasicAsciiCharacters',
    'editor.action.unicodeHighlight.showExcludeOptions',
])

const toMonacoCommand = (entry: (typeof MONACO_ACTIONS)[number]): AppCommand => ({
    id: `${MONACO_ACTION_ID_PREFIX}${entry.actionId}`,
    titleKey: `keymap.monaco.${entry.actionId}`,
    titleDefaultValue: entry.defaultLabel,
    categoryKey: entry.categoryKey,
    run: () => requestEditorPaneCommand({ type: 'run-monaco-action', actionId: entry.actionId }),
    isEnabled: MONACO_ACTION2_IDS.has(entry.actionId) ? undefined : (context) => context.activeEditorActionIds?.has(entry.actionId) ?? false,
})

export const MONACO_ACTION_COMMANDS: AppCommand[] = MONACO_ACTIONS.map(toMonacoCommand)
