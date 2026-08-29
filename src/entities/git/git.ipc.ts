import { commands } from '@shared/api/bindings'
import type { CommitOptions, DiffMode, ProjectId, TagCreateOptions } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const initGitRepository = (projectId: ProjectId) => unwrapResult(commands.gitInit(projectId))

export const getGitStatus = (projectId: ProjectId) => unwrapResult(commands.gitStatus(projectId))

export const getGitLog = (input: { projectId: ProjectId; skip: number; take: number }) =>
    unwrapResult(commands.gitLog(input.projectId, input.skip, input.take))

export const getGitRemotes = (projectId: ProjectId) => unwrapResult(commands.gitRemotes(projectId))

export const getGitDiffFile = (input: { projectId: ProjectId; path: string; mode: DiffMode; beforePath?: string | null }) =>
    unwrapResult(commands.gitDiffFile(input.projectId, input.path, input.mode, input.beforePath ?? null))

export const getGitGutter = (input: { projectId: ProjectId; path: string }) => unwrapResult(commands.gitGutter(input.projectId, input.path))

export const getGitBlameRange = (input: { projectId: ProjectId; path: string; from: number; to: number }) =>
    unwrapResult(commands.gitBlameRange(input.projectId, input.path, input.from, input.to))

export const stageGitPaths = (input: { projectId: ProjectId; paths: string[] }) => unwrapResult(commands.gitStage(input.projectId, input.paths))

export const unstageGitPaths = (input: { projectId: ProjectId; paths: string[] }) => unwrapResult(commands.gitUnstage(input.projectId, input.paths))

export const discardGitPaths = (input: { projectId: ProjectId; paths: string[] }) => unwrapResult(commands.gitDiscard(input.projectId, input.paths))

export const commitGit = (input: { projectId: ProjectId; message: string; options: CommitOptions }) =>
    unwrapResult(commands.gitCommit(input.projectId, input.message, input.options))

export const pushGit = (projectId: ProjectId) => unwrapResult(commands.gitPush(projectId))

export const pullGit = (projectId: ProjectId) => unwrapResult(commands.gitPull(projectId))

export const getGitCurrentUser = (projectId: ProjectId) => unwrapResult(commands.gitCurrentUser(projectId))

export const getGitBranches = (projectId: ProjectId) => unwrapResult(commands.gitBranches(projectId))

export const createGitBranch = (input: { projectId: ProjectId; name: string; checkout: boolean }) =>
    unwrapResult(commands.gitBranchCreate(input.projectId, input.name, input.checkout))

export const checkoutGitBranch = (input: { projectId: ProjectId; name: string }) =>
    unwrapResult(commands.gitBranchCheckout(input.projectId, input.name))

export const deleteGitBranch = (input: { projectId: ProjectId; name: string; force: boolean }) =>
    unwrapResult(commands.gitBranchDelete(input.projectId, input.name, input.force))

export const getGitStashes = (projectId: ProjectId) => unwrapResult(commands.gitStashList(projectId))

export const pushGitStash = (input: { projectId: ProjectId; message: string | null }) =>
    unwrapResult(commands.gitStashPush(input.projectId, input.message))

export const applyGitStash = (input: { projectId: ProjectId; index: number }) => unwrapResult(commands.gitStashApply(input.projectId, input.index))

export const dropGitStash = (input: { projectId: ProjectId; index: number }) => unwrapResult(commands.gitStashDrop(input.projectId, input.index))

export const discardGitHunk = (input: { projectId: ProjectId; path: string; hunkStart: number; hunkEnd: number }) =>
    unwrapResult(commands.gitDiscardHunk(input.projectId, input.path, input.hunkStart, input.hunkEnd))

export const getGitConflictSides = (input: { projectId: ProjectId; path: string }) =>
    unwrapResult(commands.gitConflictSides(input.projectId, input.path))

export const resolveGitConflict = (input: { projectId: ProjectId; path: string; content: string }) =>
    unwrapResult(commands.gitResolveConflict(input.projectId, input.path, input.content))

export const stageGitHunk = (input: { projectId: ProjectId; path: string; hunkStart: number; hunkEnd: number }) =>
    unwrapResult(commands.gitStageHunk(input.projectId, input.path, input.hunkStart, input.hunkEnd))

export const unstageGitHunk = (input: { projectId: ProjectId; path: string; hunkStart: number; hunkEnd: number }) =>
    unwrapResult(commands.gitUnstageHunk(input.projectId, input.path, input.hunkStart, input.hunkEnd))

export const stageGitLines = (input: { projectId: ProjectId; path: string; lineStart: number; lineEnd: number }) =>
    unwrapResult(commands.gitStageLines(input.projectId, input.path, input.lineStart, input.lineEnd))

export const unstageGitLines = (input: { projectId: ProjectId; path: string; lineStart: number; lineEnd: number }) =>
    unwrapResult(commands.gitUnstageLines(input.projectId, input.path, input.lineStart, input.lineEnd))

export const revertGitCommit = (input: { projectId: ProjectId; rev: string }) => unwrapResult(commands.gitRevertCommit(input.projectId, input.rev))

export const getGitTags = (projectId: ProjectId) => unwrapResult(commands.gitTags(projectId))

export const createGitTag = (input: { projectId: ProjectId; name: string; target: string; opts: TagCreateOptions }) =>
    unwrapResult(commands.gitTagCreate(input.projectId, input.name, input.target, input.opts))

export const deleteGitTag = (input: { projectId: ProjectId; name: string }) => unwrapResult(commands.gitTagDelete(input.projectId, input.name))

export const checkoutRemoteGitBranch = (input: { projectId: ProjectId; remoteRef: string }) =>
    unwrapResult(commands.gitCheckoutRemoteBranch(input.projectId, input.remoteRef))

export const getGitCommitFiles = (input: { projectId: ProjectId; rev: string }) => unwrapResult(commands.gitCommitFiles(input.projectId, input.rev))

export const getGitFileLog = (input: { projectId: ProjectId; path: string; skip: number; take: number }) =>
    unwrapResult(commands.gitFileLog(input.projectId, input.path, input.skip, input.take))

export const getGitShowFile = (input: { projectId: ProjectId; rev: string; path: string }) =>
    unwrapResult(commands.gitShowFile(input.projectId, input.rev, input.path))

export const getGitDiffStagedText = (projectId: ProjectId) => unwrapResult(commands.gitDiffStagedText(projectId))
