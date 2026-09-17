import type { FC } from 'react'
import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { FlatPaletteSymbol, PaletteLineTarget, PaletteMode } from '@shared/lib/command-palette-query'
import {
    WORKSPACE_SYMBOL_MODE_PREFIX,
    buildCommandModeQuery,
    flattenDocumentSymbols,
    parseLineModeTarget,
    parsePaletteQuery,
} from '@shared/lib/command-palette-query'
import type { ProjectId } from '@shared/api/bindings'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    formatCategorizedLabel,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    runCommandSafely,
    subscribeRegisteredCommands,
} from '@shared/lib/command-registry'
import { getActiveEditorActionIdsSnapshot, subscribeActiveEditorActionIds } from '@shared/lib/bridge/active-editor-actions-bridge'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'
import { buildKeybindingRows } from '@shared/lib/keymap/keybinding-catalog'
import { decideCommandBindingRun } from '@shared/lib/keymap/command-binding-dispatch'
import { APP_KEYMAP, MONACO_CHORD_PREFIX_KEY, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap/keymap'
import { getKeymapChordDispatchSnapshot } from '@shared/lib/keymap/keymap-chord-store'
import { deriveMonacoChordPrefixes } from '@shared/lib/monaco/monaco-keybinding'
import { IS_MAC } from '@shared/constants/platform'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { PERF_MARK, PERF_MEASURE, perfMark, perfMeasure } from '@shared/lib/perf-mark'
import { toRelativePath } from '@shared/lib/relative-path'
import { focusTextInputCaretAtEnd } from '@shared/lib/text-input-caret'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import type { NormalizedWorkspaceSymbol } from '@shared/lib/lsp/adapters/workspace-symbol'
import { createWorkspaceSymbolSearch } from '@shared/lib/lsp/adapters/workspace-symbol'
import { monaco } from '@shared/lib/monaco/setup'
import { currentWindowActiveFilePath, currentWindowFocusedPane, findActiveTab, resolveWindowPaneTree } from '@shared/lib/pane-tree'
import { useShellSlotFocus } from '@shared/lib/shell-slot-context'
import { getWindowContext } from '@shared/lib/window-context'
import { Command, CommandEmpty, CommandInput, CommandList } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { SETTINGS_JSON_TAB_TITLE } from '@shared/constants/app-file'
import { WELCOME_TAB_TITLE } from '@shared/constants/tab'
import { CommandPaletteCommandsGroup } from '@features/command-palette/command-palette-commands-group'
import { CommandPaletteFilesGroup } from '@features/command-palette/command-palette-files-group'
import { CommandPaletteLineGroup } from '@features/command-palette/command-palette-line-group'
import { CommandPaletteSymbolGroup } from '@features/command-palette/command-palette-symbol-group'
import { CommandPaletteWorkspaceSymbolGroup } from '@features/command-palette/command-palette-workspace-symbol-group'
import { fileQueryOptions } from '@entities/file/file.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { projectFilesQueryOptions } from '@entities/search/search.query'
import { layoutQueryOptions, useOpenFileTab, useOpenTab, useOpenTerminalTab, useReopenClosedTab } from '@entities/layout/layout.query'
import { revealInTab } from '@entities/editor/reveal-registry'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import type { DocumentSymbolState } from '@widgets/command-palette/use-document-symbol-loader'
import { useDocumentSymbolLoader } from '@widgets/command-palette/use-document-symbol-loader'
import type { WorkspaceSymbolState } from '@widgets/command-palette/use-workspace-symbol-search'
import { useWorkspaceSymbolSearch } from '@widgets/command-palette/use-workspace-symbol-search'

const FILE_RESULT_LIMIT = 200

const PALETTE_PLACEHOLDER_KEY: Record<PaletteMode, string> = {
    files: 'palette.filePlaceholder',
    commands: 'palette.commandPlaceholder',
    symbol: 'palette.symbolPlaceholder',
    line: 'palette.linePlaceholder',
    workspaceSymbol: 'palette.workspaceSymbolPlaceholder',
}

type CommandPaletteProps = {
    projectId: ProjectId | null
}

/**
 * Every project-scoped thing the palette does — file list, symbols, tab opens, terminal, settings —
 * is scoped to the `projectId` its host hands it, not to the global active-project session it used
 * to read itself (`activeProjectQueryOptions`). That read was the one reason the palette could only
 * be mounted in the main window (d-58 Wave I F1's open issue): an auxiliary window is pinned to its
 * own project and must never follow whatever the main window has active. The main window passes the
 * project of whichever shell slot has focus (`app/main-window-dialogs.tsx`, d-62 §1.B), an auxiliary
 * window passes its own fixed one.
 */
export const CommandPalette: FC<CommandPaletteProps> = ({ projectId }) => {
    /**
     * Whether this close was caused by the palette *acting* (running a command, opening a file,
     * revealing a symbol) rather than by Escape/outside-click. Radix restores focus to whatever was
     * focused before the dialog opened once the close animation ends, which lands *after* the
     * action already moved focus somewhere on purpose (`code-editor.tsx` focuses the editor when a
     * model is attached, a terminal tab focuses its xterm) — so a palette-opened file would end up
     * with the caret back in the previously focused surface. Only that inverted case suppresses the
     * restore; an Escape close still hands focus back where it came from.
     *
     * Disarmed on every *open* as well as in `onCloseAutoFocus`: reopening the palette (⌘P again)
     * before the previous close animation finishes means Radix never fires that close-autofocus, so
     * a flag left standing from the action-close would suppress the focus restore of the *next*
     * Escape/outside-click close — the exact focus-drops-to-body state this flag exists to avoid.
     */
    const closedByActionRef = useRef(false)
    const inputRef = useRef<HTMLInputElement>(null)

    const [open, setOpen] = useState(false)
    const [query, setQuery] = useState('')
    const [documentSymbolState, setDocumentSymbolState] = useState<DocumentSymbolState | null>(null)
    const [workspaceSymbolState, setWorkspaceSymbolState] = useState<WorkspaceSymbolState | null>(null)
    const [workspaceSymbolSearch] = useState(() => createWorkspaceSymbolSearch(monaco))

    const activeEditorActionIds = useSyncExternalStore(subscribeActiveEditorActionIds, getActiveEditorActionIdsSnapshot)
    /**
     * Read as a snapshot rather than re-listed per render: the registry only changes at bootstrap,
     * so this reference is stable for the app's lifetime and every catalog-wide derivation below
     * (`buildKeybindingRows`, the commands `fuzzyFilter`) memoizes instead of re-running on every
     * keystroke — the palette stays mounted and re-renders on each one (audit §1-14).
     */
    const registeredCommands = useSyncExternalStore(subscribeRegisteredCommands, listRegisteredCommands)

    const { t } = useTranslation()
    const { focusedShellSlotId } = useShellSlotFocus()
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mode, searchTerm } = parsePaletteQuery(query)
    const {
        data: projectFiles,
        isPending: isProjectFilesPending,
        isFetching: isProjectFilesFetching,
    } = useQuery({
        ...projectFilesQueryOptions(projectId),
        enabled: open && mode === 'files' && !!projectId,
    })
    const isSymbolNavMode = mode === 'symbol' || mode === 'line'
    const { data: layout } = useQuery({ ...layoutQueryOptions(projectId), enabled: open && isSymbolNavMode && !!projectId })
    const activePath = currentWindowActiveFilePath(layout)
    const windowPaneTree = layout ? resolveWindowPaneTree(layout, getWindowContext()) : null
    const activeTab = windowPaneTree ? findActiveTab(windowPaneTree.root, windowPaneTree.focusedPane) : null
    const activeFileTabId = activeTab?.kind.kind === 'file' ? activeTab.id : null
    const { data: activeFile } = useQuery({ ...fileQueryOptions(activePath), enabled: open && mode === 'symbol' && !!activePath })
    const { data: lspServers } = useQuery({ ...lspServersQueryOptions(), enabled: open && mode === 'symbol' })
    const needsActiveProjectRoot = mode === 'symbol' || mode === 'files'
    const { data: activeProject } = useQuery({
        ...projectQueryOptions(projectId ?? ''),
        enabled: open && needsActiveProjectRoot && !!projectId,
    })
    const { mutate: openTab } = useOpenTab(projectId)
    const openFileTab = useOpenFileTab()
    const openTerminalTab = useOpenTerminalTab(projectId)
    const { mutate: reopenClosedTabMutate } = useReopenClosedTab(projectId)

    const keymapOverrides = parseKeymapOverrides(settings?.keymapOverrides ?? null)

    const handleOpenChange = (next: boolean) => {
        setOpen(next)
        if (next) closedByActionRef.current = false
        if (!next) setQuery('')
    }

    /**
     * Every keyboard entry point into the palette goes through here so {@link closedByActionRef} is
     * disarmed on the way in — see its doc comment for the reopen-before-the-close-animation case
     * `onCloseAutoFocus` alone cannot cover.
     *
     * Also the sole start point of metric 4-a (`docs/quality-assurance/2026-09-04-perf-baseline.md`):
     * the palette has no trigger element, so `Dialog`'s `onOpenChange` never opens it — every open
     * arrives here. Only an actual open is marked; the same shortcuts pressed while the palette is
     * already up just swap the mode prefix, and timing that as an "open" would report a mode switch
     * (no mount, no dialog animation) inside metric 4-a's sample.
     */
    const openPalette = (nextQuery: string) => {
        if (!open) perfMark(PERF_MARK.PALETTE_OPEN_REQUESTED)
        closedByActionRef.current = false
        setQuery(nextQuery)
        setOpen(true)
    }

    /** Metric 4-b's start point: one keystroke in the palette input, closed by the effect below. */
    const handleQueryChange = (nextQuery: string) => {
        perfMark(PERF_MARK.PALETTE_QUERY_CHANGED)
        setQuery(nextQuery)
    }

    const closeAfterAction = () => {
        closedByActionRef.current = true
        handleOpenChange(false)
    }

    const openSettingsTab = () => {
        if (!projectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    /**
     * Passes {@link WELCOME_TAB_TITLE} rather than `t('app.welcome')` — `open_tab` activates an
     * existing `welcome` tab instead of creating a second one and leaves its title untouched, so a
     * localized title would only ever land on the very first Welcome tab and read differently from
     * the one `default_layout` seeds (see the constant's doc).
     */
    const openWelcomeTab = () => {
        if (!projectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            {
                projectId,
                kind: { kind: 'welcome' },
                title: WELCOME_TAB_TITLE,
                target: currentWindowFocusedPane(layout),
                preview: false,
            },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const openSettingsFile = () => {
        if (!projectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            {
                projectId,
                kind: { kind: 'appFile', target: { kind: 'settings' } },
                title: SETTINGS_JSON_TAB_TITLE,
                target: null,
                preview: false,
            },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const reopenClosedTab = () => {
        if (!projectId) return
        reopenClosedTabMutate(projectId, { onError: (error) => toast.error(describeIpcError(error)) })
    }

    useGlobalKeymap({
        'quick-open': () => openPalette(''),
        'command-palette': () => openPalette(buildCommandModeQuery()),
        'workspace-symbol': () => openPalette(WORKSPACE_SYMBOL_MODE_PREFIX),
        'new-terminal': openTerminalTab,
        'reopen-closed-tab': reopenClosedTab,
    })

    const commandContext: CommandContext = {
        activeProjectId: projectId,
        focusedShellSlotId,
        activeEditorActionIds,
        openSettingsTab,
        openSettingsFile,
        openTerminalTab,
        openWelcomeTab,
        reopenClosedTab,
        switchToFileSearchMode: () => setQuery(''),
    }

    const commandKeybindingRows = buildKeybindingRows(registeredCommands, keymapOverrides)

    /**
     * A second, independent `window` keydown-capture listener alongside `useGlobalKeymap`'s own
     * (`command-palette.tsx` renders both) — `runsViaCommand` rows (no `keymapId`, not a `monaco.*`
     * id) have no `APP_KEYMAP` entry for `useGlobalKeymap`/`decideKeymapDispatch` to dispatch, so
     * this is their only live-keydown path. Because it's a sibling listener on the same `window`
     * target, `useGlobalKeymap`'s `preventDefault`/`stopPropagation` never reaches it (`stopPropagation`
     * only stops propagation to other DOM nodes, not sibling listeners on the same node — see
     * `docs/features/keymap.md` §3) — so it must independently defer to the chord/monaco-deferral
     * state machine *and* to `APP_KEYMAP`'s own matching, which is what `decideCommandBindingRun`
     * shares between the two listeners (see its doc comment for the double-dispatch it closes).
     */
    useKeydownCapture((event) => {
        const row = decideCommandBindingRun({
            rows: commandKeybindingRows,
            entries: applyKeymapOverrides(APP_KEYMAP, keymapOverrides),
            event,
            chordState: getKeymapChordDispatchSnapshot(event),
            isMac: IS_MAC,
            monacoChordPrefixes: [MONACO_CHORD_PREFIX_KEY, ...deriveMonacoChordPrefixes(keymapOverrides)],
        })
        if (!row?.commandId) return
        const command = getRegisteredCommand(row.commandId)
        if (!command || !isCommandRunnable(command, commandContext)) return
        event.preventDefault()
        event.stopPropagation()
        runCommandSafely(command, commandContext)
    })

    const toProjectRelativePath = (path: string) => (activeProject ? toRelativePath(activeProject.root, path) : path)

    /**
     * `activeProject` (needed to relativize paths — `docs/acknowledge/2026-08-20-palette-ux-contract.md`
     * §1.4) can still be loading on the first open of a session where nothing else has warmed
     * `QUERY_KEY.PROJECT.DETAIL` yet. Gating file rows on it (rather than falling back to
     * `toProjectRelativePath`'s absolute-path passthrough) keeps the fuzzy match target and the
     * displayed subtitle from briefly reverting to the absolute path this feature exists to hide.
     */
    const fileProjectRootLoaded = !projectId || !!activeProject
    const documentSymbolsLoaded = documentSymbolState?.path === activePath
    const workspaceSymbolsLoaded = workspaceSymbolState?.query === searchTerm

    /**
     * Every list below is gated on the mode that actually renders it. Only one group is ever on
     * screen, but all of them used to be computed on every render — a full-project fuzzy scan
     * (`filteredFiles`, thousands of paths), a whole-catalog fuzzy scan with a `t()` label format per
     * command (`filteredCommands`), and a symbol tree flatten — including while the dialog was closed
     * (audit §1-14).
     */
    const filePaths = mode === 'files' && fileProjectRootLoaded ? (projectFiles ?? []) : []
    const filteredFiles = fuzzyFilter(searchTerm, filePaths, toProjectRelativePath).slice(0, FILE_RESULT_LIMIT)
    const filteredCommands =
        mode === 'commands'
            ? fuzzyFilter(searchTerm, registeredCommands, (command) =>
                  formatCategorizedLabel(t, command.categoryKey, command.titleKey, command.titleDefaultValue),
              )
            : []
    const flatDocumentSymbols = mode === 'symbol' && documentSymbolsLoaded ? flattenDocumentSymbols(documentSymbolState.symbols) : []
    const filteredDocumentSymbols = fuzzyFilter(searchTerm, flatDocumentSymbols, (symbol) => symbol.name)
    const workspaceSymbolResults = mode === 'workspaceSymbol' && workspaceSymbolsLoaded ? workspaceSymbolState.results : []
    const lineTarget = mode === 'line' ? parseLineModeTarget(searchTerm) : null

    const resolveEmptyStateMessage = () => {
        if (mode === 'symbol' || mode === 'line') {
            if (!activePath) return t('palette.noActiveFile')
            if (mode === 'symbol' && !documentSymbolsLoaded) return t('common.loading')
            return t('palette.noResults')
        }
        if (mode === 'files' && projectId && (!fileProjectRootLoaded || isProjectFilesPending)) return t('common.loading')
        if (mode === 'workspaceSymbol') {
            if (!projectId) return t('app.openProjectFirst')
            if (searchTerm.trim() && !workspaceSymbolsLoaded) return t('common.loading')
            return t('palette.noResults')
        }
        return t('palette.noResults')
    }

    const runCommand = (command: AppCommand) => {
        if (!isCommandRunnable(command, commandContext)) return
        runCommandSafely(command, commandContext)
        if (command.id !== 'file.quickOpen') closeAfterAction()
    }

    const openFile = (path: string) => {
        if (!projectId) return toast.info(t('app.openProjectFirst'))
        openFileTab({ projectId, path, target: null, preview: true })
        closeAfterAction()
    }

    const selectDocumentSymbol = (symbol: FlatPaletteSymbol) => {
        if (!activeFileTabId) return
        revealInTab(activeFileTabId, { line: symbol.selectionRange.startLineNumber, column: symbol.selectionRange.startColumn })
        closeAfterAction()
    }

    const selectLineTarget = (target: PaletteLineTarget) => {
        if (!activeFileTabId) return
        revealInTab(activeFileTabId, { line: target.line, column: target.column })
        closeAfterAction()
    }

    const selectWorkspaceSymbol = (symbol: NormalizedWorkspaceSymbol) => {
        if (!projectId) return
        openFileTab({ projectId, path: symbol.path, target: null, preview: true, reveal: { line: symbol.line, column: symbol.column } })
        closeAfterAction()
    }

    useDocumentSymbolLoader({
        mode,
        open,
        activeProjectId: projectId,
        activePath,
        activeFile,
        lspServers,
        activeProjectRoot: activeProject?.root,
        onLoaded: setDocumentSymbolState,
    })

    useWorkspaceSymbolSearch({
        mode,
        open,
        activeProjectId: projectId,
        searchTerm,
        workspaceSymbolSearch,
        onResult: setWorkspaceSymbolState,
    })

    /**
     * Second caret-placement path alongside `onOpenAutoFocus`, for the reopen that happens while the
     * previous close is still animating out: radix's `Presence` keeps the content mounted for the
     * exit animation, so a ⌘P/⌘⇧P/⌘T pressed inside that window flips it back to open without ever
     * remounting `FocusScope` — its mount effect, and therefore `onOpenAutoFocus`, never fires again.
     * Whatever the previous close left behind then stands: the input still carrying its select-all
     * (an Escape close never moved focus), or focus parked wherever the action that closed the
     * palette put it (`closedByActionRef`), which would leave the reopened palette unfocused.
     *
     * Runs on the first open too — after `FocusScope`'s own mount effect, since child effects flush
     * before an ancestor's — where it lands on an input `onOpenAutoFocus` already focused and
     * collapsed, and re-collapsing an unchanged caret is a no-op.
     */
    useEffect(() => {
        if (!open) return
        focusTextInputCaretAtEnd(inputRef.current)
    }, [open])

    /**
     * Closes both palette spans (metrics 4-a and 4-b) at the commit that put the result list on
     * screen — the filtering itself runs in the render body above, so an effect is the first point
     * where the work being measured is finished.
     *
     * Both are consumed here rather than in separate effects because `perfMeasure` consumes its
     * start point (`perf-mark.ts`): an open marks only 4-a's, a keystroke only 4-b's, and whichever
     * one this run does not have measures nothing instead of reporting a stale mark's age.
     */
    useEffect(() => {
        perfMeasure(PERF_MEASURE.PALETTE_OPEN, PERF_MARK.PALETTE_OPEN_REQUESTED)
        perfMeasure(PERF_MEASURE.PALETTE_FILTER, PERF_MARK.PALETTE_QUERY_CHANGED)
    }, [open, query])

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{t('palette.title')}</DialogTitle>
            </DialogHeader>
            <DialogContent
                className='overflow-hidden p-0'
                showCloseButton={false}
                onOpenAutoFocus={(event) => {
                    event.preventDefault()
                    focusTextInputCaretAtEnd(inputRef.current)
                }}
                onCloseAutoFocus={(event) => {
                    if (closedByActionRef.current) event.preventDefault()
                    closedByActionRef.current = false
                }}>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput ref={inputRef} value={query} onValueChange={handleQueryChange} placeholder={t(PALETTE_PLACEHOLDER_KEY[mode])} />
                    <CommandList>
                        <CommandEmpty>{resolveEmptyStateMessage()}</CommandEmpty>
                        {mode === 'commands' && (
                            <CommandPaletteCommandsGroup
                                commands={filteredCommands}
                                keybindingRows={commandKeybindingRows}
                                commandContext={commandContext}
                                onRunCommand={runCommand}
                            />
                        )}
                        {mode === 'files' && (
                            <CommandPaletteFilesGroup files={filteredFiles} isRefreshing={isProjectFilesFetching} onOpenFile={openFile} />
                        )}
                        {mode === 'symbol' && <CommandPaletteSymbolGroup symbols={filteredDocumentSymbols} onSelectSymbol={selectDocumentSymbol} />}
                        {mode === 'line' && (
                            <CommandPaletteLineGroup lineTarget={lineTarget} activePath={activePath} onSelectLine={selectLineTarget} />
                        )}
                        {mode === 'workspaceSymbol' && (
                            <CommandPaletteWorkspaceSymbolGroup
                                symbols={workspaceSymbolResults}
                                searchTerm={searchTerm}
                                onSelectSymbol={selectWorkspaceSymbol}
                            />
                        )}
                    </CommandList>
                </Command>
            </DialogContent>
        </Dialog>
    )
}
