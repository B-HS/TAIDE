import { commands } from '@shared/api/bindings'
import type { IdeDiagnostic, IdeDiffOutcome, IdeSelectionInput, ProjectId } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getIdeStatus = () => unwrapResult(commands.ideGetStatus())

export const startIde = () => unwrapResult(commands.ideStart())

export const stopIde = () => unwrapResult(commands.ideStop())

export const setIdeSelection = (input: IdeSelectionInput) => unwrapResult(commands.ideSetSelection(input))

export const clearIdeSelection = () => unwrapResult(commands.ideClearSelection())

export const publishIdeDiagnostics = (input: { projectId: ProjectId; items: IdeDiagnostic[] }) =>
    unwrapResult(commands.idePublishDiagnostics(input.projectId, input.items))

export const resolveIdeDiff = (input: { requestId: string; outcome: IdeDiffOutcome; content: string | null }) =>
    unwrapResult(commands.ideResolveDiff(input.requestId, input.outcome, input.content))

export const resolveIdeSave = (input: { requestId: string; saved: boolean }) => unwrapResult(commands.ideResolveSave(input.requestId, input.saved))

export const notifyIdeAtMention = (input: { path: string; lineStart: number; lineEnd: number }) =>
    unwrapResult(commands.ideNotifyAtMention(input.path, input.lineStart, input.lineEnd))
