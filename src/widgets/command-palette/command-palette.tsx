import type { FC } from 'react'
import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { languages } from 'monaco-editor'
import { Braces, CornerDownLeft, File, Hash, Terminal } from 'lucide-react'
import { toast } from 'sonner'
import type { AppCommand, CommandContext, FlatPaletteSymbol, PaletteLineTarget, PaletteMode } from '@shared/lib/command-registry'
import {
    flattenDocumentSymbols,
    formatCategorizedLabel,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    parseLineModeTarget,
    parsePaletteQuery,
} from '@shared/lib/command-registry'
import { getActiveEditorActionIdsSnapshot, subscribeActiveEditorActionIds } from '@shared/lib/active-editor-actions-bridge'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'
import { buildKeybindingRows, findKeybindingRowById, findRunnableCommandBinding } from '@shared/lib/keybinding-catalog'
import { formatKeymapShortcut, parseKeymapOverrides } from '@shared/lib/keymap'
import { getKeymapChordDispatchSnapshot } from '@shared/lib/keymap-chord-store'
import { buildFuzzyHighlightSegments, fuzzyFilter } from '@shared/lib/fuzzy-match'
import { toRelativePath } from '@shared/lib/relative-path'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { requestDocumentSymbols } from '@shared/lib/lsp/adapters/document-symbol'
import type { NormalizedWorkspaceSymbol } from '@shared/lib/lsp/adapters/workspace-symbol'
import { createWorkspaceSymbolSearch } from '@shared/lib/lsp/adapters/workspace-symbol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { buildDocumentSymbolWaiters } from '@shared/lib/lsp/document-symbol-session-waiters'
import { monaco } from '@shared/lib/monaco/setup'
import { findActiveTab } from '@shared/lib/pane-tree'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandShortcut } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { SETTINGS_JSON_TAB_TITLE } from '@shared/constants/app-file'
import { fileQueryOptions } from '@entities/file/file.query'
import { activeProjectQueryOptions, projectQueryOptions } from '@entities/project/project.query'
import { treeRowsQueryOptions } from '@entities/tree/tree.query'
import { layoutQueryOptions, useOpenTab, useReopenClosedTab } from '@entities/layout/layout.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { listSessionRecordsForProject, waitForLspSessionForRoot } from '@widgets/editor-pane/lsp-session-registry'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { splitFileMatchForDisplay } from '@widgets/command-palette/command-palette-file-match'

const FILE_RESULT_LIMIT = 200

/**
 * `--accent` (`shared/styles/global.css`) resolves to the same swatch as list hover
 * (`--taide-list-hover-background`), so the shadcn `CommandItem` base's
 * `data-[selected=true]:bg-accent` renders a selected row almost indistinguishable from the
 * palette's own panel background (contrast ratio ~1.4:1) — arrow-key selection was moving the
 * `aria-selected`/`data-selected` state correctly but rendering it near-invisibly, read by users as
 * "selection doesn't work" (`docs/acknowledge/2026-08-20-palette-ux-contract.md` §1.1). Scoped to
 * this palette only (not `shared/ui/command.tsx`, which `font-picker`/`branch-switcher`/etc. also
 * consume) and reuses the existing `--taide-list-active-background` token — the same shade already
 * proven visible elsewhere as the sidebar's "active" state (`--taide-app-sidebar-item-active`) —
 * instead of introducing a new one.
 */
const PALETTE_ITEM_SELECTED_CLASSNAME = 'data-[selected=true]:bg-list-active-background'

const HighlightedText: FC<{ text: string; indices: number[] }> = ({ text, indices }) => (
    <>
        {buildFuzzyHighlightSegments(text, indices).map((segment, index) =>
            segment.matched ? (
                <mark key={index} className='bg-panel-match-highlight text-app-background rounded-xs'>
                    {segment.text}
                </mark>
            ) : (
                segment.text
            ),
        )}
    </>
)

const PALETTE_PLACEHOLDER_KEY: Record<PaletteMode, string> = {
    files: 'palette.filePlaceholder',
    commands: 'palette.commandPlaceholder',
    symbol: 'palette.symbolPlaceholder',
    line: 'palette.linePlaceholder',
    workspaceSymbol: 'palette.workspaceSymbolPlaceholder',
}

type DocumentSymbolState = { path: string; symbols: languages.DocumentSymbol[] }
type WorkspaceSymbolState = { query: string; results: NormalizedWorkspaceSymbol[] }

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
    const { data: treePage } = useQuery({ ...treeRowsQueryOptions(activeProjectId), enabled: open && mode === 'files' && !!activeProjectId })
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
            { onError: (error) => toast.error(error.message) },
        )
    }

    const openSettingsTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
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
            { onError: (error) => toast.error(error.message) },
        )
    }

    const reopenClosedTab = () => {
        if (!activeProjectId) return
        reopenClosedTabMutate(activeProjectId, { onError: (error) => toast.error(error.message) })
    }

    useGlobalKeymap({
        'quick-open': () => {
            setQuery('')
            setOpen(true)
        },
        'command-palette': () => {
            setQuery('>')
            setOpen(true)
        },
        'workspace-symbol': () => {
            setQuery('#')
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

    const fileRows = (treePage?.rows ?? []).filter((row) => row.kind === 'file')
    const filteredFiles = fuzzyFilter(searchTerm, fileRows, (row) => toProjectRelativePath(row.path)).slice(0, FILE_RESULT_LIMIT)
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
        const name = path.slice(path.lastIndexOf('/') + 1)
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path }, title: name, target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
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
        const name = symbol.path.slice(symbol.path.lastIndexOf('/') + 1)
        requestReveal(symbol.path, symbol.line, symbol.column)
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path: symbol.path }, title: name, target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
        handleOpenChange(false)
    }

    /**
     * `⌘O`/symbol-nav mode's document-symbol lookup — root-aware (`buildDocumentSymbolWaiters` +
     * `waitForLspSessionForRoot`, contract `docs/acknowledge/2026-08-19-editor-pane-batch-contract.md`
     * §1.2) because it has a concrete `activePath` to resolve a root from. Contrast the `⌘T`
     * Workspace Symbol effect below, which stays root-agnostic (`listSessionRecordsForProject`) on
     * purpose: it has no single document path to resolve a root against (it searches every root the
     * project has open at once), so there is no "wrong root" to pick.
     */
    useEffect(() => {
        if (mode !== 'symbol' || !open || !activeProjectId || !activePath || !activeFile || !lspServers) return

        const languageId = activeFile.languageId
        const availableServerIds = lspServers
            .filter((server) => server.languageIds.includes(languageId) && server.available)
            .map((server) => server.id)

        let cancelled = false
        let pendingCancels: (() => void)[] = []

        const load = async () => {
            const waiters = await buildDocumentSymbolWaiters({
                availableServerIds,
                path: activePath,
                projectId: activeProjectId,
                fallbackRoot: activeProject?.root,
                isCancelled: () => cancelled,
                resolveRoot: resolveLspRoot,
                waitForSession: waitForLspSessionForRoot,
            })
            pendingCancels = waiters.map((waiter) => waiter.cancel)

            for (const { promise } of waiters) {
                const session = await promise
                if (!session || cancelled) continue
                const ready = await session.ready.catch(() => null)
                if (!ready || cancelled) continue
                if (!ready.client.supports((capabilities) => isCapabilityEnabled(capabilities.documentSymbolProvider))) continue
                const uri = monaco.Uri.file(activePath).toString()
                const result = await requestDocumentSymbols(monaco, ready.client, uri).catch(() => [])
                if (!cancelled) {
                    setDocumentSymbolState({ path: activePath, symbols: result })
                    return
                }
            }
            if (!cancelled) setDocumentSymbolState({ path: activePath, symbols: [] })
        }

        void load()

        return () => {
            cancelled = true
            pendingCancels.forEach((cancel) => cancel())
        }
    }, [mode, open, activeProjectId, activePath, activeFile, lspServers, activeProject?.root])

    useEffect(() => {
        if (mode !== 'workspaceSymbol' || !open || !activeProjectId) return

        const trimmedQuery = searchTerm.trim()
        let cancelled = false

        const load = async () => {
            if (!trimmedQuery) {
                setWorkspaceSymbolState({ query: searchTerm, results: [] })
                return
            }
            const sessionRecords = listSessionRecordsForProject(activeProjectId)
            const readySessions = await Promise.all(sessionRecords.map((record) => record.ready.catch(() => null)))
            if (cancelled) return
            const clients = readySessions.filter((session) => session !== null).map((session) => session.client)
            const results = await workspaceSymbolSearch.search(clients, trimmedQuery)
            if (!cancelled) setWorkspaceSymbolState({ query: searchTerm, results })
        }

        void load()

        return () => {
            cancelled = true
            workspaceSymbolSearch.cancel()
        }
    }, [mode, open, activeProjectId, searchTerm, workspaceSymbolSearch])

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
                            <CommandGroup heading={t('palette.commands')}>
                                {filteredCommands.map(({ item, match }) => {
                                    const keybindingRow = findKeybindingRowById(commandKeybindingRows, item.keymapId ?? item.id)
                                    const runnable = isCommandRunnable(item, commandContext)
                                    const label = formatCategorizedLabel(t, item.categoryKey, item.titleKey, item.titleDefaultValue)
                                    return (
                                        <CommandItem
                                            key={item.id}
                                            className={PALETTE_ITEM_SELECTED_CLASSNAME}
                                            disabled={!runnable}
                                            onSelect={() => runCommand(item)}>
                                            <Terminal className='size-4' />
                                            <span>
                                                <HighlightedText text={label} indices={match.indices} />
                                            </span>
                                            {keybindingRow?.key && <CommandShortcut>{formatKeymapShortcut(keybindingRow)}</CommandShortcut>}
                                            {!keybindingRow?.key && keybindingRow?.defaultBindingLabel && (
                                                <CommandShortcut>{keybindingRow.defaultBindingLabel}</CommandShortcut>
                                            )}
                                        </CommandItem>
                                    )
                                })}
                            </CommandGroup>
                        )}
                        {mode === 'files' && (
                            <CommandGroup heading={t('palette.files')}>
                                {filteredFiles.map(({ item, match }) => {
                                    const { fileName, dirPath, fileNameIndices, dirPathIndices } = splitFileMatchForDisplay(
                                        toProjectRelativePath(item.path),
                                        match.indices,
                                    )
                                    return (
                                        <CommandItem
                                            key={item.path}
                                            value={item.path}
                                            className={PALETTE_ITEM_SELECTED_CLASSNAME}
                                            onSelect={() => openFile(item.path)}>
                                            <File className='size-4' />
                                            <span className='flex min-w-0 flex-col'>
                                                <span className='truncate'>
                                                    <HighlightedText text={fileName} indices={fileNameIndices} />
                                                </span>
                                                {dirPath !== null && (
                                                    <span className='truncate text-xs text-muted-foreground'>
                                                        <HighlightedText text={dirPath} indices={dirPathIndices} />
                                                    </span>
                                                )}
                                            </span>
                                        </CommandItem>
                                    )
                                })}
                            </CommandGroup>
                        )}
                        {mode === 'symbol' && (
                            <CommandGroup heading={t('palette.symbols')}>
                                {filteredDocumentSymbols.map(({ item, match }) => (
                                    <CommandItem
                                        key={`${item.containerLabel}/${item.name}/${item.selectionRange.startLineNumber}`}
                                        className={PALETTE_ITEM_SELECTED_CLASSNAME}
                                        onSelect={() => selectDocumentSymbol(item)}>
                                        <Braces className='size-4' />
                                        <span className='truncate'>
                                            <HighlightedText text={item.name} indices={match.indices} />
                                        </span>
                                        {item.containerLabel && <span className='truncate text-xs text-muted-foreground'>{item.containerLabel}</span>}
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        )}
                        {mode === 'line' && lineTarget && activePath && (
                            <CommandGroup>
                                <CommandItem className={PALETTE_ITEM_SELECTED_CLASSNAME} onSelect={() => selectLineTarget(lineTarget)}>
                                    <CornerDownLeft className='size-4' />
                                    <span>{lineTarget.column > 1 ? `${lineTarget.line}:${lineTarget.column}` : `${lineTarget.line}`}</span>
                                </CommandItem>
                            </CommandGroup>
                        )}
                        {mode === 'workspaceSymbol' && (
                            <CommandGroup heading={t('palette.workspaceSymbols')}>
                                {workspaceSymbolResults.map((symbol, index) => (
                                    <CommandItem
                                        key={`${symbol.path}:${symbol.line}:${symbol.column}:${index}`}
                                        className={PALETTE_ITEM_SELECTED_CLASSNAME}
                                        onSelect={() => selectWorkspaceSymbol(symbol)}>
                                        <Hash className='size-4' />
                                        <span className='truncate'>{symbol.name}</span>
                                        {symbol.containerName && (
                                            <span className='truncate text-xs text-muted-foreground'>{symbol.containerName}</span>
                                        )}
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        )}
                    </CommandList>
                </Command>
            </DialogContent>
        </Dialog>
    )
}
