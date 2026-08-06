import { commands } from '@shared/api/bindings'
import type { ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const openFile = (path: string) => unwrapResult(commands.fileOpen(path))

export const saveFile = (input: { path: string; content: string }) => unwrapResult(commands.fileSave(input.path, input.content))

export const createEntry = (input: { path: string; isDir: boolean }) => unwrapResult(commands.fileCreate(input.path, input.isDir))

export const renameEntry = (input: { from: string; to: string }) => unwrapResult(commands.fileRename(input.from, input.to))

export const deleteEntry = (path: string) => unwrapResult(commands.fileDelete(path))

export const copyEntry = (input: { from: string; to: string }) => unwrapResult(commands.fileCopy(input.from, input.to))

export const mirrorDirty = (input: { projectId: ProjectId; path: string; content: string }) =>
    unwrapResult(commands.fileMirrorDirty(input.projectId, input.path, input.content))
