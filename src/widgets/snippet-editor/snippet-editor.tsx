import type { FC } from 'react'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { useDeleteSnippet, useSaveSnippet, useSnippetList } from '@entities/snippet/snippet.query'
import { IpcError } from '@shared/api/unwrap-result'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { isGlobalSnippetFileName } from '@shared/lib/snippet-file'
import {
    appendSnippetEntryDraft,
    draftsToSnippetContent,
    findIncompleteSnippetEntryDrafts,
    hasDuplicateSnippetEntryNames,
    hasUnsavedSnippetDraftChanges,
    removeSnippetEntryDraft,
    snippetMapToDrafts,
    updateSnippetEntryDraft,
    type SnippetEntryDraft,
} from '@shared/lib/snippet-draft'
import { NewSnippetFileDialog } from '@widgets/snippet-editor/new-snippet-file-dialog'
import { SnippetEntryEditor } from '@features/snippet/snippet-entry-editor'
import { SnippetFileList } from '@features/snippet/snippet-file-list'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { Button } from '@shared/ui/button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

type SnippetEditorProps = { onClose: () => void }

/** Where the editor was heading when unsaved drafts stopped it — replayed once the user confirms the discard. */
type PendingDiscardTarget = { kind: 'selectFile'; fileName: string } | { kind: 'close' }

export const SnippetEditor: FC<SnippetEditorProps> = ({ onClose }) => {
    const { t } = useTranslation()

    const [selectedFileName, setSelectedFileName] = useState<string | null>(null)
    const [syncedFileName, setSyncedFileName] = useState<string | null>(null)
    const [draftEntries, setDraftEntries] = useState<SnippetEntryDraft[] | null>(null)
    const [newFileDialogOpen, setNewFileDialogOpen] = useState(false)
    const [deleteFileOpen, setDeleteFileOpen] = useState(false)
    const [deleteEntryId, setDeleteEntryId] = useState<string | null>(null)
    const [pendingDiscardTarget, setPendingDiscardTarget] = useState<PendingDiscardTarget | null>(null)

    const { data: files = [] } = useSnippetList()
    const { mutate: saveSnippetMutate, isPending: isSaving } = useSaveSnippet()
    const { mutate: deleteSnippetMutate, isPending: isDeleting } = useDeleteSnippet()

    const selectedFile = files.find((file) => file.fileName === selectedFileName) ?? null

    if (selectedFileName !== syncedFileName) {
        setSyncedFileName(selectedFileName)
        setDraftEntries(null)
    } else if (!draftEntries && selectedFile) {
        setDraftEntries(snippetMapToDrafts(selectedFile.snippets))
    }

    const showScope = selectedFileName !== null && isGlobalSnippetFileName(selectedFileName)
    const pendingDeleteEntryName = draftEntries?.find((draft) => draft.id === deleteEntryId)?.name ?? ''
    const hasUnsavedChanges = Boolean(selectedFile && draftEntries && hasUnsavedSnippetDraftChanges(draftEntries, selectedFile.snippets))

    /**
     * Selecting another file resets `draftEntries` (the render-time sync above) and leaving unmounts
     * the whole editor — both threw away unsaved snippet edits with no prompt at all (audit §4-B D6).
     * The navigation is parked here instead of performed, and replayed by
     * {@link handleConfirmDiscardChanges} only if the user confirms.
     */
    const requestSelectFile = (fileName: string) => {
        if (fileName === selectedFileName) return
        if (hasUnsavedChanges) {
            setPendingDiscardTarget({ kind: 'selectFile', fileName })
            return
        }
        setSelectedFileName(fileName)
    }

    const requestClose = () => {
        if (hasUnsavedChanges) {
            setPendingDiscardTarget({ kind: 'close' })
            return
        }
        onClose()
    }

    const handleConfirmDiscardChanges = () => {
        if (!pendingDiscardTarget) return
        setPendingDiscardTarget(null)
        if (pendingDiscardTarget.kind === 'close') {
            onClose()
            return
        }
        setSelectedFileName(pendingDiscardTarget.fileName)
    }

    /**
     * `handleCreateFile`'s `content` is always the literal `'{}'`, so an `InvalidArgument` there can
     * only be the file name Rust's `sanitize_snippet_file_name` rejected — never a JSON parse
     * failure. `handleSave`'s `content` is always `JSON.stringify` output for the same reason, so
     * its `InvalidArgument` is the mirror case. Reporting both under the same "invalid JSON" message
     * pointed users at the wrong field for the only failure either path can actually produce.
     */
    const handleCreateFileError = (error: unknown) =>
        toast.error(
            error instanceof IpcError && error.code === 'InvalidArgument' ? t('snippetEditor.invalidFileName') : t('snippetEditor.saveFailed'),
        )

    const handleSaveError = (error: unknown) =>
        toast.error(error instanceof IpcError && error.code === 'InvalidArgument' ? t('snippetEditor.parseError') : t('snippetEditor.saveFailed'))

    const handleCreateFile = (fileName: string) => {
        setNewFileDialogOpen(false)
        saveSnippetMutate({ fileName, content: '{}' }, { onSuccess: () => requestSelectFile(fileName), onError: handleCreateFileError })
    }

    const handleSave = () => {
        if (!selectedFileName || !draftEntries) return
        const incompleteDrafts = findIncompleteSnippetEntryDrafts(draftEntries)
        if (incompleteDrafts.length > 0) {
            toast.error(t('snippetEditor.incompleteEntryError', { count: incompleteDrafts.length }))
            return
        }
        if (hasDuplicateSnippetEntryNames(draftEntries)) {
            toast.error(t('snippetEditor.duplicateNameError'))
            return
        }
        saveSnippetMutate(
            { fileName: selectedFileName, content: draftsToSnippetContent(draftEntries) },
            { onSuccess: () => toast.success(t('snippetEditor.saveSuccess')), onError: handleSaveError },
        )
    }

    const handleDeleteFile = () => {
        if (!selectedFileName) return
        deleteSnippetMutate(selectedFileName, {
            onSuccess: () => {
                setDeleteFileOpen(false)
                setSelectedFileName(null)
            },
            onError: (error) => toast.error(describeIpcError(error)),
        })
    }

    const handleConfirmDeleteEntry = () => {
        if (deleteEntryId) setDraftEntries((current) => (current ? removeSnippetEntryDraft(current, deleteEntryId) : current))
        setDeleteEntryId(null)
    }

    return (
        <div className='bg-app-background text-app-foreground flex h-full w-full flex-col overflow-hidden'>
            <div className='border-app-border flex items-center justify-between gap-4 border-b px-6 py-4'>
                <div className='flex items-center gap-3'>
                    <Button variant='ghost' size='sm' onClick={requestClose}>
                        {t('snippetEditor.backToSettings')}
                    </Button>
                    {selectedFileName && <span className='text-app-foreground text-sm font-medium'>{selectedFileName}</span>}
                </div>
                {selectedFileName && (
                    <div className='flex items-center gap-2'>
                        <Button variant='outline' size='sm' onClick={() => setDeleteFileOpen(true)} disabled={isDeleting}>
                            {t('snippetEditor.deleteFileButton')}
                        </Button>
                        <Button size='sm' onClick={handleSave} disabled={isSaving}>
                            {t('snippetEditor.save')}
                        </Button>
                    </div>
                )}
            </div>

            <div className='flex min-h-0 flex-1'>
                <ScrollContainer className='border-app-border w-64 shrink-0 border-r' viewportClassName='px-4 py-4'>
                    <div className='mb-2 flex items-center justify-between gap-2'>
                        <span className='text-app-sidebar-icon-default text-xs font-medium'>{t('snippetEditor.fileListTitle')}</span>
                        <Button variant='outline' size='xs' onClick={() => setNewFileDialogOpen(true)}>
                            <Plus className='size-3.5' />
                            {t('snippetEditor.newFileButton')}
                        </Button>
                    </div>
                    <SnippetFileList files={files} selectedFileName={selectedFileName} onSelect={requestSelectFile} />
                </ScrollContainer>

                <ScrollContainer className='min-w-0 flex-1' viewportClassName='px-6 py-4'>
                    {!selectedFileName || !draftEntries ? (
                        <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.noFiles')}</span>
                    ) : (
                        <div className='flex flex-col gap-3'>
                            <div className='flex items-center justify-between gap-2'>
                                <span className='text-app-sidebar-icon-default text-xs font-medium'>{t('snippetEditor.snippetListTitle')}</span>
                                <Button variant='outline' size='xs' onClick={() => setDraftEntries(appendSnippetEntryDraft(draftEntries))}>
                                    <Plus className='size-3.5' />
                                    {t('snippetEditor.addSnippetButton')}
                                </Button>
                            </div>
                            {draftEntries.length === 0 ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.noSnippets')}</span>
                            ) : (
                                draftEntries.map((draft) => (
                                    <SnippetEntryEditor
                                        key={draft.id}
                                        draft={draft}
                                        showScope={showScope}
                                        onChange={(patch) => setDraftEntries(updateSnippetEntryDraft(draftEntries, draft.id, patch))}
                                        onDelete={() => setDeleteEntryId(draft.id)}
                                    />
                                ))
                            )}
                        </div>
                    )}
                </ScrollContainer>
            </div>

            <NewSnippetFileDialog
                open={newFileDialogOpen}
                existingFileNames={files.map((file) => file.fileName)}
                onOpenChange={setNewFileDialogOpen}
                onCreate={handleCreateFile}
            />

            <AlertDialog open={deleteFileOpen} onOpenChange={setDeleteFileOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('snippetEditor.deleteFileConfirmTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('snippetEditor.deleteFileConfirmDescription', { fileName: selectedFileName ?? '' })}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={handleDeleteFile}>
                            {t('snippetEditor.deleteFileButton')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <AlertDialog open={pendingDiscardTarget !== null} onOpenChange={(open) => !open && setPendingDiscardTarget(null)}>
                <AlertDialogContent size='sm'>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.unsavedChangesTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>{t('common.unsavedChangesDescription')}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={handleConfirmDiscardChanges}>
                            {t('common.discardChanges')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <AlertDialog open={deleteEntryId !== null} onOpenChange={(open) => !open && setDeleteEntryId(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('snippetEditor.deleteConfirmTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('snippetEditor.deleteConfirmDescription', { name: pendingDeleteEntryName })}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={handleConfirmDeleteEntry}>
                            {t('snippetEditor.deleteSnippetButton')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    )
}
