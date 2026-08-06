import { commands } from '@shared/api/bindings'
import type { CommitOptions, DiffMode, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getGitStatus = (projectId: ProjectId) => unwrapResult(commands.gitStatus(projectId))

export const getGitLog = (input: { projectId: ProjectId; skip: number; take: number }) =>
    unwrapResult(commands.gitLog(input.projectId, input.skip, input.take))

export const getGitRemotes = (projectId: ProjectId) => unwrapResult(commands.gitRemotes(projectId))

export const getGitDiffFile = (input: { projectId: ProjectId; path: string; mode: DiffMode }) =>
    unwrapResult(commands.gitDiffFile(input.projectId, input.path, input.mode))

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
