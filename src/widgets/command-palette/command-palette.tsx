import { useState, useSyncExternalStore } from 'react'
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
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { formatCategorizedLabel, getRegisteredCommand, isCommandRunnable, listRegisteredCommands } from '@shared/lib/command-registry'
import { getActiveEditorActionIdsSnapshot, subscribeActiveEditorActionIds } from '@shared/lib/bridge/active-editor-actions-bridge'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'
import { buildKeybindingRows, findRunnableCommandBinding } from '@shared/lib/keymap/keybinding-catalog'
import { parseKeymapOverrides } from '@shared/lib/keymap/keymap'
import { getKeymapChordDispatchSnapshot } from '@shared/lib/keymap/keymap-chord-store'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import type { NormalizedWorkspaceSymbol } from '@shared/lib/lsp/adapters/workspace-symbol'
import { createWorkspaceSymbolSearch } from '@shared/lib/lsp/adapters/workspace-symbol'
import { monaco } from '@shared/lib/monaco/setup'
import { findActiveTab } from '@shared/lib/pane-tree'
import { Command, CommandEmpty, CommandInput, CommandList } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { SETTINGS_JSON_TAB_TITLE } from '@shared/constants/app-file'
import { CommandPaletteCommandsGroup } from '@features/command-palette/command-palette-commands-group'
import { CommandPaletteFilesGroup } from '@features/command-palette/command-palette-files-group'
import { CommandPaletteLineGroup } from '@features/command-palette/command-palette-line-group'
import { CommandPaletteSymbolGroup } from '@features/command-palette/command-palette-symbol-group'
import { CommandPaletteWorkspaceSymbolGroup } from '@features/command-palette/command-palette-workspace-symbol-group'
import { fileQueryOptions } from '@entities/file/file.query'
import { activeProjectQueryOptions, projectQueryOptions } from '@entities/project/project.query'
import { projectFilesQueryOptions } from '@entities/search/search.query'
import { layoutQueryOptions, useOpenTab, useReopenClosedTab } from '@entities/layout/layout.query'
import { requestReveal } from '@entities/editor/reveal-registry'
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

export const CommandPalette = () => {
    const [open, setOpen] = useState(false)
    const [query, setQuery] = useState('')
    const [documentSymbolState, setDocumentSymbolState] = useState<DocumentSymbolState | null>(null)
    const [workspaceSymbolState, setWorkspaceSymbolState] = useState<WorkspaceSymbolState | null>(null)
    const [workspaceSymbolSearch] = useState(() => createWorkspaceSymbolSearch(monaco))

    const activeEditorActionIds = useSyncExternalStore(subscribeActiveEditorActionIds, getActiveEditorActionIdsSnapshot)

    const { t } = useTranslation()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mode, searchTerm } = parsePaletteQuery(query)
    const { data: projectFiles, isPending: isProjectFilesPending } = useQuery({
        ...projectFilesQueryOptions(activeProjectId),
        enabled: open && mode === 'files' && !!activeProjectId,
    })
    const isSymbolNavMode = mode === 'symbol' || mode === 'line'
    const { data: layout } = useQuery({ ...layoutQueryOptions(activeProjectId), enabled: open && isSymbolNavMode && !!activeProjectId })
    const activeTab = layout ? findActiveTab(layout.root, layout.focusedPane) : null
    const activePath = activeTab?.kind.kind === 'file' ? activeTab.kind.path : null
    const { data: activeFile } = useQuery({ ...fileQueryOptions(activePath), enabled: open && mode === 'symbol' && !!activePath })
    const { data: lspServers } = useQuery({ ...lspServersQueryOptions(), enabled: open && mode === 'symbol' })
    const needsActiveProjectRoot = mode === 'symbol' || mode === 'files'
    const { data: activeProject } = useQuery({
        ...projectQueryOptions(activeProjectId ?? ''),
        enabled: open && needsActiveProjectRoot && !!activeProjectId,
    })
    const { mutate: openTab } = useOpenTab(activeProjectId)
    const { mutate: reopenClosedTabMutate } = useReopenClosedTab(activeProjectId)

    const keymapOverrides = parseKeymapOverrides(settings?.keymapOverrides ?? null)

    const handleOpenChange = (next: boolean) => {
        setOpen(next)
        if (!next) setQuery('')
    }

    const openTerminalTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: null, preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const openSettingsTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const openSettingsFile = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            {
                projectId: activeProjectId,
                kind: { kind: 'appFile', target: { kind: 'settings' } },
                title: SETTINGS_JSON_TAB_TITLE,
                target: null,
                preview: false,
            },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const reopenClosedTab = () => {
        if (!activeProjectId) return
        reopenClosedTabMutate(activeProjectId, { onError: (error) => toast.error(describeIpcError(error)) })
    }

    useGlobalKeymap({
        'quick-open': () => {
            setQuery('')
            setOpen(true)
        },
        'command-palette': () => {
            setQuery(buildCommandModeQuery())
            setOpen(true)
        },
        'workspace-symbol': () => {
            setQuery(WORKSPACE_SYMBOL_MODE_PREFIX)
            setOpen(true)
        },
        'new-terminal': openTerminalTab,
        'reopen-closed-tab': reopenClosedTab,
    })

    const commandContext: CommandContext = {
        activeProjectId,
        activeEditorActionIds,
        openSettingsTab,
        openSettingsFile,
        openTerminalTab,
        reopenClosedTab,
        switchToFileSearchMode: () => setQuery(''),
    }

    const commandKeybindingRows = buildKeybindingRows(listRegisteredCommands(), keymapOverrides)

    /**
     * A second, independent `window` keydown-capture listener alongside `useGlobalKeymap`'s own
     * (`command-palette.tsx` renders both) — `runsViaCommand` rows (no `keymapId`, not a `monaco.*`
     * id) have no `APP_KEYMAP` entry for `useGlobalKeymap`/`decideKeymapDispatch` to dispatch, so
     * this is their only live-keydown path. Because it's a sibling listener on the same `window`
     * target, `useGlobalKeymap`'s `preventDefault`/`stopPropagation` never reaches it (`stopPropagation`
     * only stops propagation to other DOM nodes, not sibling listeners on the same node — see
     * `docs/features/keymap.md` §3) — so it must independently defer to the chord/monaco-deferral
     * state machine itself. Without this check, a `runsViaCommand` row rebound to a key that
     * collides with a chord's 2nd stage or a monaco-deferred keydown would fire *underneath* that
     * state machine: the "2단은 무조건 삼킨다" mis-input guard and the monaco chord yield window
     * would both leak past this listener (Wave H contract §3.1).
     */
    useKeydownCapture((event) => {
        const chordState = getKeymapChordDispatchSnapshot(event)
        if (chordState.pending || chordState.monacoDeferral) return
        const row = findRunnableCommandBinding(commandKeybindingRows, event)
        if (!row?.commandId) return
        const command = getRegisteredCommand(row.commandId)
        if (!command || !isCommandRunnable(command, commandContext)) return
        event.preventDefault()
        event.stopPropagation()
        void command.run(commandContext)
    })

    const toProjectRelativePath = (path: string) => (activeProject ? toRelativePath(activeProject.root, path) : path)

    /**
     * `activeProject` (needed to relativize paths — `docs/acknowledge/2026-08-20-palette-ux-contract.md`
     * §1.4) can still be loading on the first open of a session where nothing else has warmed
     * `QUERY_KEY.PROJECT.DETAIL` yet. Gating file rows on it (rather than falling back to
     * `toProjectRelativePath`'s absolute-path passthrough) keeps the fuzzy match target and the
     * displayed subtitle from briefly reverting to the absolute path this feature exists to hide.
     */
    const fileProjectRootLoaded = !activeProjectId || !!activeProject
    const filePaths = fileProjectRootLoaded ? (projectFiles ?? []) : []
    const filteredFiles = fuzzyFilter(searchTerm, filePaths, toProjectRelativePath).slice(0, FILE_RESULT_LIMIT)
    const filteredCommands = fuzzyFilter(searchTerm, listRegisteredCommands(), (command) =>
        formatCategorizedLabel(t, command.categoryKey, command.titleKey, command.titleDefaultValue),
    )

    const documentSymbolsLoaded = documentSymbolState?.path === activePath
    const flatDocumentSymbols = documentSymbolsLoaded ? flattenDocumentSymbols(documentSymbolState.symbols) : []
    const filteredDocumentSymbols = fuzzyFilter(searchTerm, flatDocumentSymbols, (symbol) => symbol.name)

    const workspaceSymbolsLoaded = workspaceSymbolState?.query === searchTerm
    const workspaceSymbolResults = workspaceSymbolsLoaded ? workspaceSymbolState.results : []

    const lineTarget = parseLineModeTarget(searchTerm)

    const resolveEmptyStateMessage = () => {
        if (mode === 'symbol' || mode === 'line') {
            if (!activePath) return t('palette.noActiveFile')
            if (mode === 'symbol' && !documentSymbolsLoaded) return t('common.loading')
            return t('palette.noResults')
        }
        if (mode === 'files' && activeProjectId && (!fileProjectRootLoaded || isProjectFilesPending)) return t('common.loading')
        if (mode === 'workspaceSymbol') {
            if (!activeProjectId) return t('app.openProjectFirst')
            if (searchTerm.trim() && !workspaceSymbolsLoaded) return t('common.loading')
            return t('palette.noResults')
        }
        return t('palette.noResults')
    }

    const runCommand = (command: AppCommand) => {
        if (!isCommandRunnable(command, commandContext)) return
        void command.run(commandContext)
        if (command.id !== 'file.quickOpen') handleOpenChange(false)
    }

    const openFile = (path: string) => {
        if (!activeProjectId) return
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
        handleOpenChange(false)
    }

    const selectDocumentSymbol = (symbol: FlatPaletteSymbol) => {
        if (!activePath) return
        requestReveal(activePath, symbol.selectionRange.startLineNumber, symbol.selectionRange.startColumn)
        handleOpenChange(false)
    }

    const selectLineTarget = (target: PaletteLineTarget) => {
        if (!activePath) return
        requestReveal(activePath, target.line, target.column)
        handleOpenChange(false)
    }

    const selectWorkspaceSymbol = (symbol: NormalizedWorkspaceSymbol) => {
        if (!activeProjectId) return
        requestReveal(symbol.path, symbol.line, symbol.column)
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path: symbol.path }, title: fileNameOf(symbol.path), target: null, preview: true },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
        handleOpenChange(false)
    }

    useDocumentSymbolLoader({
        mode,
        open,
        activeProjectId,
        activePath,
        activeFile,
        lspServers,
        activeProjectRoot: activeProject?.root,
        onLoaded: setDocumentSymbolState,
    })

    useWorkspaceSymbolSearch({
        mode,
        open,
        activeProjectId,
        searchTerm,
        workspaceSymbolSearch,
        onResult: setWorkspaceSymbolState,
    })

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{t('palette.title')}</DialogTitle>
            </DialogHeader>
            <DialogContent className='overflow-hidden p-0' showCloseButton={false}>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput value={query} onValueChange={setQuery} placeholder={t(PALETTE_PLACEHOLDER_KEY[mode])} />
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
                            <CommandPaletteFilesGroup files={filteredFiles} toProjectRelativePath={toProjectRelativePath} onOpenFile={openFile} />
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
