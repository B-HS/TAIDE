import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId, ProjectLayout, TabId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { QUERY_KEY } from '@shared/constants/query-key'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { fileQueryOptions } from '@entities/file/file.query'
import { useCloseTab } from '@entities/layout/layout.query'
import { useResolveIdeDiff } from '@entities/ide/ide.query'
import { getPendingClaudeDiff, removePendingClaudeDiff, takePendingClaudeDiffIfUnresolved } from '@entities/ide/claude-diff-registry'
import { Button } from '@shared/ui/button'

const FALLBACK_LANGUAGE_ID = 'plaintext'

type ClaudeDiffPaneProps = {
    projectId: ProjectId
    tabId: TabId
    requestId: string
    path: string
}

/**
 * Whether `tabId` is still open somewhere in the project — any pane, any window, active or not.
 * Must search every window's tree (`collectAllPaneTabs`), not just the main one: a claudeDiff tab
 * always opens in the main window (`ide-sync-provider.tsx`), but its tab-bar context menu can move
 * it into an auxiliary window like any other tab (`layout_move_tab_to_window` doesn't filter by tab
 * kind) — that move still unmounts this pane from the main window, and at that point the tab lives
 * only under `layout.auxiliaryWindows[].root`, not `layout.root`. A single-tree check would read that
 * as "closed" and fire an implicit reject for a tab the user never rejected.
 */
export const isTabStillOpenInLayout = (layout: ProjectLayout | undefined, tabId: TabId): boolean =>
    !!layout && collectAllPaneTabs(layout).some((tab) => tab.id === tabId)

export const ClaudeDiffPane: FC<ClaudeDiffPaneProps> = ({ projectId, tabId, requestId, path }) => {
    const { t } = useTranslation()
    const containerRef = useRef<HTMLDivElement>(null)
    const diffEditorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null)
    const originalModelRef = useRef<monaco.editor.ITextModel | null>(null)
    const modifiedModelRef = useRef<monaco.editor.ITextModel | null>(null)
    const requestedContentsRef = useRef(getPendingClaudeDiff(requestId)?.newContents ?? '')
    const [isResolving, setIsResolving] = useState(false)

    const queryClient = useQueryClient()
    const { data: file } = useQuery({ ...fileQueryOptions(path), retry: false })
    const { mutateAsync: resolveDiff } = useResolveIdeDiff()
    const { mutate: closeTab } = useCloseTab(projectId)

    const languageId = file?.languageId ?? FALLBACK_LANGUAGE_ID
    const originalContent = file?.content ?? ''

    useEffect(() => {
        if (!containerRef.current) return

        const diffEditor = monaco.editor.createDiffEditor(containerRef.current, { automaticLayout: true, readOnly: false })
        const originalModel = monaco.editor.createModel('', FALLBACK_LANGUAGE_ID)
        const modifiedModel = monaco.editor.createModel(requestedContentsRef.current, FALLBACK_LANGUAGE_ID)
        diffEditor.setModel({ original: originalModel, modified: modifiedModel })
        diffEditorRef.current = diffEditor
        originalModelRef.current = originalModel
        modifiedModelRef.current = modifiedModel

        return () => {
            diffEditor.setModel(null)
            diffEditorRef.current = null
            originalModelRef.current = null
            modifiedModelRef.current = null
            diffEditor.dispose()
            originalModel.dispose()
            modifiedModel.dispose()
        }
    }, [])

    useEffect(() => {
        const originalModel = originalModelRef.current
        const modifiedModel = modifiedModelRef.current
        if (!originalModel || !modifiedModel) return

        if (originalModel.getValue() !== originalContent) originalModel.setValue(originalContent)
        monaco.editor.setModelLanguage(originalModel, languageId)
        monaco.editor.setModelLanguage(modifiedModel, languageId)
    }, [originalContent, languageId])

    /**
     * Releases the backend's pending `ide:diff-requested` wait when this pane goes away without the
     * user ever clicking Accept/Reject — e.g. closing the tab directly from the tab bar. This pane
     * only renders while its tab is the *active* tab of its pane (`pane-node-view.tsx`), so a plain
     * unmount effect can't tell "the user switched to another tab, this one is still open" apart
     * from "the tab actually closed" — `isTabStillOpenInLayout` (see its own doc for why it has to
     * search every window's tree) is what makes that distinction. The layout cache the tab-close/move
     * mutation writes (`layout.query.ts`'s `useLayoutMutation`) is already updated by the time this
     * pane actually unmounts as a result of it — the cache write is what triggers the re-render that
     * removes this pane from the tree in the first place — so this stays correct even through React
     * 18 `StrictMode`'s (`main.tsx`) synchronous mount→cleanup→remount replay: at that replay's
     * cleanup, nothing has changed yet, so the tab is still found and this is a no-op, same as a real
     * "switched to another tab, this one is still open" case.
     */
    useEffect(() => {
        return () => {
            const layout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(projectId))
            if (isTabStillOpenInLayout(layout, tabId)) return
            if (!takePendingClaudeDiffIfUnresolved(requestId)) return
            void resolveDiff({ requestId, outcome: 'rejected', content: null }).catch(() => undefined)
        }
    }, [requestId, tabId, projectId, queryClient, resolveDiff])

    const handleAccept = async () => {
        if (isResolving) return
        setIsResolving(true)
        const content = diffEditorRef.current?.getModifiedEditor().getValue() ?? requestedContentsRef.current
        try {
            await resolveDiff({ requestId, outcome: 'saved', content })
            removePendingClaudeDiff(requestId)
            toast.success(t('ide.diffAccepted'))
            closeTab(tabId)
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error))
        } finally {
            setIsResolving(false)
        }
    }

    const handleReject = async () => {
        if (isResolving) return
        setIsResolving(true)
        try {
            await resolveDiff({ requestId, outcome: 'rejected', content: null })
            removePendingClaudeDiff(requestId)
            toast.info(t('ide.diffRejected'))
            closeTab(tabId)
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error))
        } finally {
            setIsResolving(false)
        }
    }

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border bg-editor-background flex h-8 shrink-0 items-center justify-end gap-2 border-b px-2'>
                <Button variant='outline' size='xs' disabled={isResolving} onClick={() => void handleReject()}>
                    {t('ide.rejectChanges')}
                </Button>
                <Button variant='default' size='xs' disabled={isResolving} onClick={() => void handleAccept()}>
                    {t('ide.acceptChanges')}
                </Button>
            </div>
            <div ref={containerRef} className='min-h-0 flex-1' />
        </div>
    )
}
