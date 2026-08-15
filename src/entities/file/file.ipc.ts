import { commands } from '@shared/api/bindings'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const openFile = (path: string) => unwrapResult(commands.fileOpen(path))

export const saveFile = (input: { path: string; content: string }) => unwrapResult(commands.fileSave(input.path, input.content))

export const createEntry = (input: { path: string; isDir: boolean }) => unwrapResult(commands.fileCreate(input.path, input.isDir))

export const renameEntry = (input: { from: string; to: string }) => unwrapResult(commands.fileRename(input.from, input.to))

export const deleteEntry = (path: string) => unwrapResult(commands.fileDelete(path))

export const copyEntry = (input: { from: string; to: string }) => unwrapResult(commands.fileCopy(input.from, input.to))

/** Returns the disk `modifiedMs` baseline the backend derived live at write time (see `file/service.rs`'s `mirror_dirty` doc comment) — never caller-supplied. */
export const mirrorDirty = (input: { projectId: ProjectId; path: string; content: string }) =>
    unwrapResult(commands.fileMirrorDirty(input.projectId, input.path, input.content))

export const listMirrors = (projectId: ProjectId) => unwrapResult(commands.fileListMirrors(projectId))

export const clearMirror = (input: { projectId: ProjectId; path: string }) => unwrapResult(commands.fileClearMirror(input.projectId, input.path))

export const pruneMirrors = (input: { projectId: ProjectId; keepPaths: string[] }) =>
    unwrapResult(commands.filePruneMirrors(input.projectId, input.keepPaths))

export const mirrorUntitled = (input: { projectId: ProjectId; tabId: TabId; content: string }) =>
    unwrapResult(commands.fileMirrorUntitled(input.projectId, input.tabId, input.content))

export const listUntitledMirrors = (projectId: ProjectId) => unwrapResult(commands.fileListUntitledMirrors(projectId))

export const clearUntitledMirror = (input: { projectId: ProjectId; tabId: TabId }) =>
    unwrapResult(commands.fileClearUntitledMirror(input.projectId, input.tabId))

export const pruneUntitledMirrors = (input: { projectId: ProjectId; keepTabIds: TabId[] }) =>
    unwrapResult(commands.filePruneUntitledMirrors(input.projectId, input.keepTabIds))

export const flushMirrorsComplete = () => unwrapResult(commands.fileFlushComplete())
