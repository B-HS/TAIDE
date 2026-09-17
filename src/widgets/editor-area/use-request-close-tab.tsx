import { useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { save } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import type { ProjectId, Tab, TabId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { getSaveRequest } from '@entities/editor/save-request-registry'
import { disposeModel, getModel, toUntitledModelPath } from '@entities/editor/model-registry'
import { dropUntitledContent, getUntitledContent } from '@entities/editor/untitled-registry'
import { clearUntitledMirror } from '@entities/file/file.ipc'
import { fileMirrorsQueryOptions, untitledMirrorsQueryOptions, useSaveFile } from '@entities/file/file.query'
import { useCloseTab, useConvertUntitledTab, useSetTabDirty } from '@entities/layout/layout.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { CloseDirtyTabDialog } from '@features/tab/close-dirty-tab-dialog'
import { closeTabsSerially } from '@widgets/editor-area/close-tabs-serially'

type PendingClose = { tabs: Tab[]; dirtyTabs: Tab[] }

/**
 * How many close confirmations are on screen across every instance of this hook. One `EditorArea`
 * and each of its `PaneTabBar`s own their own copy, and ⌘W is dispatched from a document-level
 * keydown capture that a modal dialog does not stop — so without a shared count, pressing ⌘W while
 * the tab bar's "Close All" question was up would stack a second dialog on the first. A plain module
 * counter is enough because every writer is a React effect in the same realm, and the effect's
 * cleanup returns it to zero on unmount.
 */
let openConfirmationCount = 0

const isDirtyGatedTab = (tab: Tab) => !!tab.dirty && (tab.kind.kind === 'file' || tab.kind.kind === 'untitled')

/**
 * The one door every tab close goes through — the tab's ✕, its context menu, a middle click, ⌘W, the
 * four "Close Others/To the Right/Saved/All" loops and ⌘K ⌘W — so the unsaved-changes question
 * `docs/features/tabs.md` §8 requires exists in exactly one place instead of at six call sites that
 * each used to publish `layout_close_tab` unconditionally.
 *
 * Why closing is worth a question at all: `useCloseTab`'s `releaseClosedFileTabPath` clears the
 * file's hot-exit mirror and disposes its monaco model, and Rust's closed-tab stack keeps only tab
 * metadata — so one click took the edit *and* both ways back (audit wave 2 #8).
 *
 * A request carrying several dirty tabs asks once and applies the answer to all of them, the way VS
 * Code does; the caller decides which tabs are in the request (pinned tabs are filtered out before
 * they get here, as they always were).
 *
 * - **Save** writes each dirty tab, then closes — and waits for each write to actually land first.
 *   A write that fails, a read-only file that refuses one, or a Save As the user backs out of cancels
 *   the close, because a close after a failed save is the data loss the dialog exists to prevent.
 * - **Don't save** clears `dirty` server-side *before* closing, so the tab does not enter the closed
 *   stack carrying a dirty flag that ⌘⇧T would then paint as a ghost dot (audit wave 2 #21).
 * - **Cancel** does nothing at all.
 *
 * Terminal, diff, settings and every other kind close immediately, as before: only file and untitled
 * tabs hold unsaved text.
 */
export const useRequestCloseTab = (projectId: ProjectId) => {
    const [pendingClose, setPendingClose] = useState<PendingClose | null>(null)

    const queryClient = useQueryClient()
    const { t } = useTranslation()
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: mirrors } = useQuery(fileMirrorsQueryOptions(projectId))
    const { data: untitledMirrors } = useQuery(untitledMirrorsQueryOptions(projectId))
    const { mutateAsync: closeTabAsync } = useCloseTab(projectId)
    const { mutateAsync: setTabDirtyAsync } = useSetTabDirty(projectId)
    const { mutateAsync: saveFileAsync } = useSaveFile(projectId)
    const { mutateAsync: convertUntitledAsync } = useConvertUntitledTab(projectId)

    /**
     * The live buffer first: a model outlives the pane that mounted it, so a background tab's edits
     * are still readable here. The hot-exit mirror is the fallback for the one case with no model at
     * all — a tab restored from a previous session and never opened since.
     */
    const readFileDraft = (path: string) => getModel(path)?.getValue() ?? mirrors?.find((entry) => entry.path === path)?.content ?? null

    const readUntitledDraft = (tabId: TabId) =>
        getModel(toUntitledModelPath(tabId))?.getValue() ??
        getUntitledContent(projectId, tabId) ??
        untitledMirrors?.find((entry) => entry.tabId === tabId)?.content ??
        null

    /** The cleanup `UntitledPane.handleConvertSuccess` performs after its own Save As — the draft is a file now, so the in-memory copy and the untitled mirror behind it are obsolete. */
    const releaseUntitledDraft = (tabId: TabId) => {
        dropUntitledContent(projectId, tabId)
        disposeModel(toUntitledModelPath(tabId))
        void clearUntitledMirror({ projectId, tabId }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId) })
    }

    /**
     * An untitled tab has no path, so its save is a Save As — run here rather than through the tab's
     * own pane so the answer is observable (the pane's handler reports neither the write nor the
     * user backing out of the native dialog) and so it works for a tab that is not mounted at all.
     */
    const saveUntitledTab = async (tab: Tab) => {
        const defaultPath = project ? `${project.root}/${tab.title}` : undefined
        const selected = await save({ defaultPath, title: t('tab.saveAsTitle') })
        if (!selected) return false

        await saveFileAsync({ path: selected, content: readUntitledDraft(tab.id) ?? '' })
        await convertUntitledAsync({ tabId: tab.id, path: selected })
        releaseUntitledDraft(tab.id)
        return true
    }

    /**
     * A mounted file tab saves through its own pane's pipeline, which is the only path that honours
     * the read-only refusal, Code Actions on Save, format-on-save and the on-save whitespace
     * cleanups — reached through `save-request-registry` rather than through the pane's
     * `taide.saveFile` monaco action, because an action's `run()` resolves without saying whether
     * the write landed and this answer decides whether the tab closes. An unmounted tab has no
     * pipeline to run, so it goes straight to `file_save` with the draft — and a tab with neither a
     * model nor a mirror has nothing left to write, which counts as saved.
     */
    const saveDirtyTab = async (tab: Tab) => {
        if (tab.kind.kind === 'untitled') return saveUntitledTab(tab)
        if (tab.kind.kind !== 'file') return true

        const requestSave = getSaveRequest(tab.id)
        if (requestSave) return requestSave()

        const content = readFileDraft(tab.kind.path)
        if (content === null) return true
        await saveFileAsync({ path: tab.kind.path, content })
        return true
    }

    const runClose = async (tabs: Tab[]) => {
        const failures = await closeTabsSerially(
            tabs.map((tab) => tab.id),
            closeTabAsync,
        )
        if (failures.length > 0) toast.error(describeIpcError(failures[0]))
    }

    /** Best effort, and deliberately so: the point is to keep `push_closed` from stacking a dirty flag, and a tab that answers `NotFound` here is already gone. */
    const markClean = (tabs: Tab[]) => Promise.all(tabs.map((tab) => setTabDirtyAsync({ tabId: tab.id, dirty: false }).catch(() => undefined)))

    const requestCloseTabs = (tabs: Tab[]) => {
        if (openConfirmationCount > 0) return
        const dirtyTabs = tabs.filter(isDirtyGatedTab)
        if (dirtyTabs.length === 0) {
            void runClose(tabs)
            return
        }
        setPendingClose({ tabs, dirtyTabs })
    }

    const handleSave = async (pending: PendingClose) => {
        setPendingClose(null)
        for (const tab of pending.dirtyTabs) {
            try {
                if (!(await saveDirtyTab(tab))) return
            } catch (error) {
                toast.error(describeIpcError(error))
                return
            }
        }
        await runClose(pending.tabs)
    }

    const handleDiscard = async (pending: PendingClose) => {
        setPendingClose(null)
        await markClean(pending.dirtyTabs)
        await runClose(pending.tabs)
    }

    useEffect(() => {
        if (!pendingClose) return
        openConfirmationCount += 1
        return () => {
            openConfirmationCount -= 1
        }
    }, [pendingClose])

    return {
        requestCloseTab: (tab: Tab) => requestCloseTabs([tab]),
        requestCloseTabs,
        /** ⌘W and ⌘K ⌘W ride a document-level keydown capture the dialog does not block, so their handlers ask this before firing again. */
        isCloseConfirmationOpen: () => openConfirmationCount > 0,
        closeDirtyTabDialog: (
            <CloseDirtyTabDialog
                dirtyTitles={(pendingClose?.dirtyTabs ?? []).map((tab) => tab.title)}
                onSave={() => pendingClose && void handleSave(pendingClose)}
                onDiscard={() => pendingClose && void handleDiscard(pendingClose)}
                onCancel={() => setPendingClose(null)}
            />
        ),
    }
}
