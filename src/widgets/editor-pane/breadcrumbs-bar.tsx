import type { FC } from 'react'
import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { languages } from 'monaco-editor'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { requestDocumentSymbols } from '@shared/lib/lsp/adapters/document-symbol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'
import {
    buildSegmentPaths,
    filterDirectChildren,
    findEnclosingSymbolChain,
    parentDirOf,
    splitRelativePathSegments,
    type CursorPosition,
} from '@widgets/editor-pane/breadcrumb-path'
import { waitForLspSession } from '@widgets/editor-pane/lsp-session-registry'
import { getEditorInstance, subscribeEditorInstance } from '@entities/editor/editor-instance-registry'
import { fileQueryOptions } from '@entities/file/file.query'
import { layoutQueryOptions, useOpenTab } from '@entities/layout/layout.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import { treeRowsQueryOptions, useRevealTreeNode } from '@entities/tree/tree.query'
import type { BreadcrumbSegmentEntry } from '@features/editor/breadcrumb-segment'
import { BreadcrumbSegment } from '@features/editor/breadcrumb-segment'

type SymbolsForPath = { path: string; symbols: languages.DocumentSymbol[] }

type RenderedSegment = {
    key: string
    label: string
    interactive: boolean
    entries: BreadcrumbSegmentEntry[]
    onOpenChange?: (open: boolean) => void
}

type BreadcrumbsBarProps = {
    projectId: ProjectId
    tabId: TabId | null
    path: string | null
}

export const BreadcrumbsBar: FC<BreadcrumbsBarProps> = ({ projectId, tabId, path }) => {
    const cursorSnapshotRef = useRef<CursorPosition | null>(null)

    const [symbolsForPath, setSymbolsForPath] = useState<SymbolsForPath | null>(null)

    const { t } = useTranslation()
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: file } = useQuery(fileQueryOptions(path))
    const { data: servers } = useQuery(lspServersQueryOptions())
    const { data: treeRowPage } = useQuery(treeRowsQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: revealTreeNode } = useRevealTreeNode(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const languageId = file?.languageId ?? null

    const relativePath = project && path ? toRelativePath(project.root, path) : null
    const pathSegments = relativePath ? splitRelativePathSegments(relativePath) : []
    const segmentPaths = project ? buildSegmentPaths(project.root, pathSegments) : []
    const trimmedRoot = segmentPaths.length > 0 ? parentDirOf(segmentPaths[0]) : (project?.root ?? '')
    const treeRows = treeRowPage?.rows ?? []
    const symbols = symbolsForPath?.path === path ? symbolsForPath.symbols : []

    const getCursorSnapshot = () => {
        const editor = tabId ? getEditorInstance(tabId) : null
        const position = editor?.getPosition()
        if (!position) {
            cursorSnapshotRef.current = null
            return null
        }
        const cached = cursorSnapshotRef.current
        if (cached && cached.lineNumber === position.lineNumber && cached.column === position.column) return cached
        const next = { lineNumber: position.lineNumber, column: position.column }
        cursorSnapshotRef.current = next
        return next
    }

    const subscribeToCursor = (onStoreChange: () => void) => {
        if (!tabId) return () => {}

        let cursorSubscription: { dispose: () => void } | null = null

        const attachToEditor = () => {
            cursorSubscription?.dispose()
            const editor = getEditorInstance(tabId)
            cursorSubscription = editor?.onDidChangeCursorPosition(onStoreChange) ?? null
            onStoreChange()
        }

        attachToEditor()
        const editorSubscription = subscribeEditorInstance(tabId, attachToEditor)

        return () => {
            editorSubscription()
            cursorSubscription?.dispose()
        }
    }

    const cursorPosition = useSyncExternalStore(subscribeToCursor, getCursorSnapshot)
    const enclosingChain = cursorPosition ? findEnclosingSymbolChain(symbols, cursorPosition) : []

    const handleOpenFile = (targetPath: string) => {
        requestReveal(targetPath, 1, 1)
        openTab(
            {
                projectId,
                kind: { kind: 'file', path: targetPath },
                title: fileNameOf(targetPath),
                target: currentWindowFocusedPane(layout),
                preview: true,
            },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const handleSelectSymbol = (target: languages.DocumentSymbol) => {
        if (!path) return
        requestReveal(path, target.selectionRange.startLineNumber, target.selectionRange.startColumn)
    }

    const handlePathDropdownOpenChange = (open: boolean) => {
        if (open && path) revealTreeNode({ projectId, path })
    }

    const pathRenderedSegments: RenderedSegment[] = pathSegments.map((segmentName, index) => {
        const parentPath = index === 0 ? trimmedRoot : segmentPaths[index - 1]
        const siblings = filterDirectChildren(treeRows, parentPath)
        return {
            key: `path-${index}`,
            label: segmentName,
            interactive: true,
            onOpenChange: handlePathDropdownOpenChange,
            entries: siblings.map((row) => ({
                key: row.path,
                label: row.name,
                disabled: row.kind === 'directory',
                onSelect: row.kind === 'directory' ? undefined : () => handleOpenFile(row.path),
            })),
        }
    })

    const symbolRenderedSegments: RenderedSegment[] = enclosingChain.map((target, index) => {
        const siblings = index === 0 ? symbols : (enclosingChain[index - 1].children ?? [])
        return {
            key: `symbol-${index}-${target.name}`,
            label: target.name,
            interactive: siblings.length > 1,
            entries: siblings.map((sibling, siblingIndex) => ({
                key: `${sibling.name}-${siblingIndex}`,
                label: sibling.name,
                onSelect: () => handleSelectSymbol(sibling),
            })),
        }
    })

    const renderedSegments = [...pathRenderedSegments, ...symbolRenderedSegments]

    useEffect(() => {
        if (!path || !languageId || !servers) return

        const availableServerIds = servers.filter((server) => server.languageIds.includes(languageId) && server.available).map((server) => server.id)
        if (availableServerIds.length === 0) return

        let cancelled = false
        const waiters = availableServerIds.map((serverId) => waitForLspSession(projectId, serverId))

        const load = async () => {
            for (const { promise } of waiters) {
                const session = await promise
                if (!session || cancelled) continue

                const ready = await session.ready.catch(() => null)
                if (!ready || cancelled) continue
                if (!ready.client.supports((capabilities) => isCapabilityEnabled(capabilities.documentSymbolProvider))) continue

                const uri = monaco.Uri.file(path).toString()
                const result = await requestDocumentSymbols(monaco, ready.client, uri).catch(() => [])
                if (!cancelled) {
                    setSymbolsForPath({ path, symbols: result })
                    return
                }
            }
            if (!cancelled) setSymbolsForPath({ path, symbols: [] })
        }

        void load()

        return () => {
            cancelled = true
            waiters.forEach(({ cancel }) => cancel())
        }
    }, [path, languageId, servers, projectId])

    return (
        <nav
            aria-label={t('breadcrumbs.title')}
            className='border-app-border bg-editor-background flex h-8 shrink-0 items-center gap-0.5 overflow-x-auto border-b px-2'>
            {!path || !tabId ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('breadcrumbs.noActiveFile')}</span>
            ) : (
                renderedSegments.map((segment, index) => (
                    <BreadcrumbSegment
                        key={segment.key}
                        label={segment.label}
                        emphasized={index === renderedSegments.length - 1}
                        interactive={segment.interactive}
                        entries={segment.entries}
                        dropdownAriaLabel={t('breadcrumbs.dropdownAriaLabel')}
                        showSeparator={index > 0}
                        onOpenChange={segment.onOpenChange}
                    />
                ))
            )}
        </nav>
    )
}
