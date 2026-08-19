import { commands } from '@shared/api/bindings'
import type { DropEdge, PaneId, ProjectId, ShellViewPatch, TabId, TabKind, TabWindowTarget } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getLayout = (projectId: ProjectId) => unwrapResult(commands.layoutGet(projectId))

export const openTab = (input: { projectId: ProjectId; kind: TabKind; title: string; target: PaneId | null; preview: boolean }) =>
    unwrapResult(commands.layoutOpenTab(input.projectId, input.kind, input.title, input.target, input.preview))

export const closeTab = (tabId: TabId) => unwrapResult(commands.layoutCloseTab(tabId))

export const activateTab = (tabId: TabId) => unwrapResult(commands.layoutActivateTab(tabId))

export const moveTab = (input: { tabId: TabId; paneId: PaneId; index: number }) =>
    unwrapResult(commands.layoutMoveTab(input.tabId, input.paneId, input.index))

export const splitPane = (input: { paneId: PaneId; edge: DropEdge; tabId: TabId }) =>
    unwrapResult(commands.layoutSplit(input.paneId, input.edge, input.tabId))

export const resizePane = (input: { paneId: PaneId; sizes: number[] }) => unwrapResult(commands.layoutResize(input.paneId, input.sizes))

export const focusPane = (paneId: PaneId) => unwrapResult(commands.layoutFocusPane(paneId))

export const pinTab = (input: { tabId: TabId; pinned: boolean }) => unwrapResult(commands.layoutPinTab(input.tabId, input.pinned))

export const setTabPreview = (input: { tabId: TabId; preview: boolean }) => unwrapResult(commands.layoutSetPreview(input.tabId, input.preview))

export const setTabDirty = (input: { tabId: TabId; dirty: boolean }) => unwrapResult(commands.layoutSetDirty(input.tabId, input.dirty))

export const setTerminalSession = (input: { tabId: TabId; sessionId: string }) =>
    unwrapResult(commands.layoutSetTerminalSession(input.tabId, input.sessionId))

export const reopenClosedTab = (projectId: ProjectId) => unwrapResult(commands.layoutReopenClosed(projectId))

export const openUntitledTab = (input: { projectId: ProjectId; target: PaneId | null }) =>
    unwrapResult(commands.layoutOpenUntitled(input.projectId, input.target))

export const convertUntitledTab = (input: { tabId: TabId; path: string }) => unwrapResult(commands.layoutConvertUntitled(input.tabId, input.path))

export const moveTabToWindow = (input: { tabId: TabId; target: TabWindowTarget }) =>
    unwrapResult(commands.layoutMoveTabToWindow(input.tabId, input.target))

export const setShellView = (input: { projectId: ProjectId; patch: ShellViewPatch }) =>
    unwrapResult(commands.layoutSetShellView(input.projectId, input.patch))

export const setTabViewState = (input: { tabId: TabId; viewState: string | null }) =>
    unwrapResult(commands.layoutSetViewState(input.tabId, input.viewState))
