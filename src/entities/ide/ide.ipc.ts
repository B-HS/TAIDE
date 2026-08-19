import { getCurrentWindow } from '@tauri-apps/api/window'
import { commands } from '@shared/api/bindings'
import type { IdeDiagnostic, IdeDiffOutcome, IdeSelectionInput, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getIdeStatus = () => unwrapResult(commands.ideGetStatus())

/**
 * `owner` (this window's OS label) is injected here so a remote session's fixed `REMOTE_OWNER_LABEL`
 * selection can never overwrite a desktop window's selection — see `IdeSelectionInput.owner`'s Rust
 * doc comment (R6#12).
 */
export const setIdeSelection = (input: Omit<IdeSelectionInput, 'owner'>) =>
    unwrapResult(commands.ideSetSelection({ ...input, owner: getCurrentWindow().label }))

export const clearIdeSelection = () => unwrapResult(commands.ideClearSelection(getCurrentWindow().label))

export const publishIdeDiagnostics = (input: { projectId: ProjectId; items: IdeDiagnostic[] }) =>
    unwrapResult(commands.idePublishDiagnostics(input.projectId, input.items))

export const resolveIdeDiff = (input: { requestId: string; outcome: IdeDiffOutcome; content: string | null }) =>
    unwrapResult(commands.ideResolveDiff(input.requestId, input.outcome, input.content))

export const resolveIdeSave = (input: { requestId: string; saved: boolean }) => unwrapResult(commands.ideResolveSave(input.requestId, input.saved))
