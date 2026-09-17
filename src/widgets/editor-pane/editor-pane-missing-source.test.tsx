import { describe, expect, mock, test } from 'bun:test'
import type { MirrorEntry } from '@shared/api/bindings'
import { TooltipProvider } from '@shared/ui/tooltip'
import { renderWithProviders, screen, waitFor } from '@shared/testing/render'

/**
 * d-67 #10 — a file deleted outside the app (`rm`, a branch switch) makes `file_open` fail, and the
 * pane used to stop at a bare error line. The hot-exit draft was on disk the whole time, reachable
 * from nowhere, and closing the broken-looking tab deleted it. What is locked here is the branch:
 * the draft is shown with a way to save it, and only when Rust actually reports the mirror as
 * `source_missing` — an ordinary open failure must keep its plain error screen.
 *
 * `@shared/lib/monaco/setup` is faked because `editor-pane.tsx` reaches it at import time, and
 * `@entities/file/file.ipc` is both the failure source (`openFile`) and the draft source
 * (`listMirrors`). `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3), so the file.ipc fake covers that module's entire export
 * surface rather than the two entries this test reads.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const MISSING_PATH = '/repo/gone.ts'
const DRAFT_CONTENT = 'the only copy of this work'

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

const mirrors: { current: MirrorEntry[] } = { current: [] }

mock.module('@entities/file/file.ipc', () => ({
    openFile: () => Promise.reject(new Error('file not found')),
    saveFile: rejectLikeUnavailableIpc,
    createEntry: rejectLikeUnavailableIpc,
    renameEntry: rejectLikeUnavailableIpc,
    deleteEntry: rejectLikeUnavailableIpc,
    copyEntry: rejectLikeUnavailableIpc,
    mirrorDirty: rejectLikeUnavailableIpc,
    listMirrors: () => Promise.resolve(mirrors.current),
    clearMirror: () => Promise.resolve(null),
    pruneMirrors: rejectLikeUnavailableIpc,
    mirrorUntitled: rejectLikeUnavailableIpc,
    listUntitledMirrors: () => Promise.resolve([]),
    clearUntitledMirror: rejectLikeUnavailableIpc,
    pruneUntitledMirrors: rejectLikeUnavailableIpc,
    flushMirrorsComplete: rejectLikeUnavailableIpc,
}))

const importEditorPane = () => import('@widgets/editor-pane/editor-pane')

const buildMissingSourceMirror = (): MirrorEntry => ({
    path: MISSING_PATH,
    content: DRAFT_CONTENT,
    savedAtMs: 1_700_000_000_000,
    diskModifiedMs: null,
    conflict: false,
    sourceMissing: true,
})

const renderPane = async () => {
    const { EditorPane } = await importEditorPane()
    renderWithProviders(
        <TooltipProvider>
            <EditorPane projectId='project-1' tabId='tab-1' path={MISSING_PATH} autoFocus={false} />
        </TooltipProvider>,
    )
}

describe('EditorPane 원본이 사라진 초안', () => {
    test('source_missing 미러가 있으면 초안 본문과 다른 이름으로 저장을 보여준다', async () => {
        mirrors.current = [buildMissingSourceMirror()]

        await renderPane()

        await waitFor(() => expect(screen.getByText('editor.sourceDeleted')).toBeTruthy())
        expect(screen.getByText(DRAFT_CONTENT)).toBeTruthy()
        expect(screen.getByRole('button', { name: 'editor.saveDraftAs' })).toBeTruthy()
    })

    test('미러가 없으면 종전대로 열기 실패 화면만 보여준다', async () => {
        mirrors.current = []

        await renderPane()

        await waitFor(() => expect(screen.queryByText('editor.saveDraftAs')).toBeNull())
        expect(screen.queryByText('editor.sourceDeleted')).toBeNull()
    })

    test('원본이 멀쩡한 미러(source_missing 아님)는 배너를 띄우지 않는다', async () => {
        mirrors.current = [{ ...buildMissingSourceMirror(), sourceMissing: false, diskModifiedMs: 1 }]

        await renderPane()

        await waitFor(() => expect(screen.queryByText('editor.saveDraftAs')).toBeNull())
        expect(screen.queryByText(DRAFT_CONTENT)).toBeNull()
    })
})
