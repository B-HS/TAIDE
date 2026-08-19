import type { CancellationToken, languages } from 'monaco-editor'
import { toast } from 'sonner'
import type { LspServerId } from '@shared/api/bindings'
import type { LspClient } from '@shared/lib/lsp/client'
import type { LspCommand } from '@shared/lib/lsp/command-relay'
import { executeLspCommand } from '@shared/lib/lsp/command-relay'
import { getStoredDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'
import { i18next } from '@shared/i18n/i18n'
import { kindMatchesAny } from '@shared/lib/lsp/kind'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Diagnostic, LspRange, ServerCapabilities, WorkspaceEdit } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import type { WorkspaceEditApplyResult } from '@shared/lib/lsp/workspace-edit-applier'
import { applyWorkspaceEdit } from '@shared/lib/lsp/workspace-edit-applier'

const NOOP_DISPOSABLE = { dispose: () => {} }

/** An LSP `CodeAction` (the richer half of `textDocument/codeAction`'s `(Command | CodeAction)[]` response). */
export type LspCodeAction = {
    title: string
    kind?: string
    diagnostics?: Diagnostic[]
    isPreferred?: boolean
    disabled?: { reason: string }
    edit?: WorkspaceEdit
    command?: LspCommand
    data?: unknown
}

/** Either half of the LSP `textDocument/codeAction` response union. */
export type LspCodeActionOrCommand = LspCommand | LspCodeAction

/** A bare `Command` has `command` as a top-level *string*; a `CodeAction`'s `command` (if any) is a nested object. Unambiguous per spec — only one shape has this field at all. */
export const isLspCommandOnly = (item: LspCodeActionOrCommand): item is LspCommand => typeof (item as LspCommand).command === 'string'

const FALLBACK_CODE_ACTION_KINDS = ['quickfix', 'refactor', 'source'] as const

/** `codeActionProvider: true` (boolean) tells us nothing about which kinds the server produces — falls back to the three top-level LSP kinds so on-save/menu `providedCodeActionKinds` gating still lets something through. */
const resolveProvidedCodeActionKinds = (capability: ServerCapabilities['codeActionProvider']): string[] =>
    typeof capability === 'object' && capability.codeActionKinds?.length ? capability.codeActionKinds : [...FALLBACK_CODE_ACTION_KINDS]

/** Whether the server declared `codeActionProvider.resolveProvider` — gates the `codeAction/resolve` round-trip in {@link resolveEffectiveCodeAction}. Exported so on-save (editor-pane.tsx) can compute it once per server, same as the interactive provider does. */
export const supportsCodeActionResolve = (client: LspClient) =>
    client.supports((capabilities) => typeof capabilities.codeActionProvider === 'object' && capabilities.codeActionProvider.resolveProvider === true)

const rangesIntersect = (a: LspRange, b: LspRange) => {
    const aStartsAfterB = a.start.line > b.end.line || (a.start.line === b.end.line && a.start.character > b.end.character)
    const aEndsBeforeB = a.end.line < b.start.line || (a.end.line === b.start.line && a.end.character < b.start.character)
    return !aStartsAfterB && !aEndsBeforeB
}

const toMonacoCommandShape = (command: LspCommand): languages.Command => ({ id: command.command, title: command.title, arguments: command.arguments })

/**
 * LSP `CodeActionTriggerKind` (`Invoked = 1`, `Automatic = 2`) numerically matches monaco's own
 * `CodeActionTriggerType` (`Invoke = 1`, `Auto = 2`), so `context.trigger` is forwarded as-is —
 * see the `as const` cast at the call site instead of a conversion table.
 */
const CODE_ACTION_TRIGGER_KIND = { INVOKED: 1, AUTOMATIC: 2 } as const

/**
 * Resolves the effective `CodeAction` for `item` — a network round-trip via `codeAction/resolve`
 * when the server declared `resolveProvider` *and* the item still lacks `edit` (LSP 3.17 and
 * monaco's own `resolveCodeAction` both gate purely on "no `edit` yet", not on whether `data` is
 * present — `data` is optional metadata a server *may* attach to help it resolve, not a
 * precondition for resolving at all). Falls back to `item` unchanged on any resolve failure, so a
 * transient error degrades to "nothing to apply" rather than throwing.
 */
export const resolveEffectiveCodeAction = async (client: LspClient, supportsResolve: boolean, item: LspCodeAction): Promise<LspCodeAction> => {
    if (!supportsResolve || item.edit) return item
    try {
        return await client.request<LspCodeAction>('codeAction/resolve', item)
    } catch {
        return item
    }
}

/** Signature of `executeLspCommand`, factored out purely for test injection (real usage always defaults to it). */
export type LspCommandExecutor = (command: LspCommand) => Promise<unknown>

/**
 * Applies one `Command | CodeAction` end to end: resolve (if needed) → apply its `edit` (via
 * {@link applyWorkspaceEdit}, so unopened files and resource operations work) → run its `command`
 * only once the edit succeeded (LSP spec: edit before command). Used by Code Actions on Save,
 * which drives the LSP client directly and has no monaco `CodeActionController` to hand the
 * command off to — unlike {@link registerCodeAction}'s `resolveCodeAction`, which leaves running
 * `.command` to monaco itself so its own command-failure notification stays intact.
 */
export const applyCodeActionOrCommand = async (
    monaco: Monaco,
    client: LspClient,
    supportsResolve: boolean,
    item: LspCodeActionOrCommand,
    executeCommand: LspCommandExecutor = executeLspCommand,
): Promise<WorkspaceEditApplyResult> => {
    if (isLspCommandOnly(item)) {
        try {
            await executeCommand(item)
            return { applied: true }
        } catch (error) {
            return { applied: false, failureReason: error instanceof Error ? error.message : String(error) }
        }
    }

    const effective = await resolveEffectiveCodeAction(client, supportsResolve, item)
    const applyResult = effective.edit
        ? await applyWorkspaceEdit(monaco, effective.edit, undefined, { getDocumentVersion: client.getDocumentVersion })
        : { applied: true }
    if (!applyResult.applied || !effective.command) return applyResult

    await executeCommand(effective.command).catch(() => undefined)
    return applyResult
}

/**
 * Fetches `textDocument/codeAction` restricted to a single `kind` (Code Actions on Save issues one
 * request per kind — `source.fixAll` then `source.organizeImports` — rather than one combined
 * request, matching VS Code's own save-participant behavior). Post-filters the response by kind
 * hierarchy since servers are not required to honor `only` strictly (LSP spec).
 */
export const requestCodeActionsForKind = async (
    client: LspClient,
    uri: string,
    range: LspRange,
    diagnostics: readonly Diagnostic[],
    kind: string,
): Promise<LspCodeActionOrCommand[]> => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.codeActionProvider))) return []
    const result = await client.request<LspCodeActionOrCommand[] | null>('textDocument/codeAction', {
        textDocument: { uri },
        range,
        context: { diagnostics, only: [kind], triggerKind: CODE_ACTION_TRIGGER_KIND.AUTOMATIC },
    })
    return (result ?? []).filter((item) => isLspCommandOnly(item) || !item.kind || kindMatchesAny([kind], item.kind))
}

type CodeActionListToken = { disposed: boolean }

type StoredCodeAction = { raw: LspCodeActionOrCommand; listToken: CodeActionListToken }

const storedByAction = new WeakMap<languages.CodeAction, StoredCodeAction>()

const toMonacoCodeAction = (item: LspCodeActionOrCommand, listToken: CodeActionListToken): languages.CodeAction => {
    const action: languages.CodeAction = isLspCommandOnly(item)
        ? { title: item.title, command: toMonacoCommandShape(item) }
        : { title: item.title, kind: item.kind, isPreferred: item.isPreferred, disabled: item.disabled?.reason }
    storedByAction.set(action, { raw: item, listToken })
    return action
}

/**
 * `CodeActionProvider.resolveCodeAction` — monaco invokes this for *every* selected action whose
 * `edit` is still unset (`Will only invoked when missing`, per monaco.d.ts), which is every action
 * {@link toMonacoCodeAction} produces, deliberately: it is the only public hook that fires between
 * "user picked this action" and monaco's own `bulkEditService.apply`, so it is where our own
 * {@link applyWorkspaceEdit} runs *instead of* monaco's (which throws for files with no open
 * model). Returning with `edit` still unset makes monaco's subsequent `bulkEditService.apply` a
 * no-op (`item.action.edit?.edits.length` is falsy); mutating `action.command` in place is what
 * then lets monaco run the command itself — but only when the edit actually applied, and never
 * for a stale (post-`didChange`) or disposed list.
 */
export const resolveMonacoCodeAction = async (
    monaco: Monaco,
    client: LspClient,
    supportsResolve: boolean,
    action: languages.CodeAction,
    token: CancellationToken,
): Promise<languages.CodeAction> => {
    const stored = storedByAction.get(action)
    if (!stored || isLspCommandOnly(stored.raw) || stored.listToken.disposed || token.isCancellationRequested) return action

    const effective = await resolveEffectiveCodeAction(client, supportsResolve, stored.raw)
    if (stored.listToken.disposed || token.isCancellationRequested) return action

    if (effective.edit) {
        const result = await applyWorkspaceEdit(monaco, effective.edit, undefined, { getDocumentVersion: client.getDocumentVersion })
        if (!result.applied) {
            toast.error(i18next.t('editor.workspaceEditApplyFailed'))
            return action
        }
    }
    if (effective.command) action.command = toMonacoCommandShape(effective.command)
    return action
}

/**
 * Registers the LSP CodeAction provider for `languageId` on `serverId`'s session. Needs `serverId`
 * (unlike most adapters, which only take `languageId`) to read that server's own diagnostics back
 * out of `diagnostics.ts`'s side map for `context.diagnostics` — same reason `registerDiagnostics`
 * itself is not part of `LANGUAGE_ADAPTER_REGISTRARS`.
 */
export const registerCodeAction = (monaco: Monaco, client: LspClient, serverId: LspServerId, languageId: string) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.codeActionProvider))) return NOOP_DISPOSABLE

    const providedCodeActionKinds = resolveProvidedCodeActionKinds(client.getCapabilities()?.codeActionProvider)
    const supportsResolve = supportsCodeActionResolve(client)

    const provider: languages.CodeActionProvider = {
        provideCodeActions: async (model, range, context, token) => {
            const uri = model.uri.toString()
            const lspRange = monacoRangeToLsp(range)
            const diagnostics = getStoredDiagnostics(serverId, uri, client).filter((diagnostic) => rangesIntersect(diagnostic.range, lspRange))

            let raw: LspCodeActionOrCommand[] | null
            try {
                raw = await client.request<LspCodeActionOrCommand[] | null>('textDocument/codeAction', {
                    textDocument: { uri },
                    range: lspRange,
                    context: { diagnostics, only: context.only ? [context.only] : undefined, triggerKind: context.trigger },
                })
            } catch {
                raw = null
            }
            if (token.isCancellationRequested || !raw) return { actions: [], dispose: () => {} }

            const listToken: CodeActionListToken = { disposed: false }
            const contentChangeDisposable = model.onDidChangeContent(() => {
                listToken.disposed = true
            })

            return {
                actions: raw.map((item) => toMonacoCodeAction(item, listToken)),
                dispose: () => {
                    listToken.disposed = true
                    contentChangeDisposable.dispose()
                },
            }
        },
        resolveCodeAction: (action, token) => resolveMonacoCodeAction(monaco, client, supportsResolve, action, token),
    }

    return monaco.languages.registerCodeActionProvider(languageId, provider, { providedCodeActionKinds })
}
