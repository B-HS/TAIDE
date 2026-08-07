import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { fileQueryOptions } from '@entities/file/file.query'
import { useCloseTab } from '@entities/layout/layout.query'
import { useResolveIdeDiff } from '@entities/ide/ide.query'
import { getPendingClaudeDiff, removePendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { Button } from '@shared/ui/button'

const FALLBACK_LANGUAGE_ID = 'plaintext'

type ClaudeDiffPaneProps = {
    projectId: ProjectId
    tabId: TabId
    requestId: string
    path: string
}

export const ClaudeDiffPane: FC<ClaudeDiffPaneProps> = ({ projectId, tabId, requestId, path }) => {
    const { t } = useTranslation()
    const containerRef = useRef<HTMLDivElement>(null)
    const diffEditorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null)
    const originalModelRef = useRef<monaco.editor.ITextModel | null>(null)
    const modifiedModelRef = useRef<monaco.editor.ITextModel | null>(null)
    const requestedContentsRef = useRef(getPendingClaudeDiff(requestId)?.newContents ?? '')
    const [isResolving, setIsResolving] = useState(false)

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
