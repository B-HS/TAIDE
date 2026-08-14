import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { WorkspaceEdit } from '@shared/lib/lsp/protocol'
import { registerServerRequestHandler, type ServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'
import { applyWorkspaceEdit } from '@shared/lib/lsp/workspace-edit-applier'

type ApplyWorkspaceEditParams = { label?: string; edit: WorkspaceEdit }

const isApplyWorkspaceEditParams = (params: unknown): params is ApplyWorkspaceEditParams =>
    typeof params === 'object' && params !== null && 'edit' in params

/**
 * A `failureReason` bound for `applyWorkspaceEdit` can carry a local absolute filesystem path
 * (e.g. "already exists: /Users/alice/project/out.ts") — useful for a local UI (rename's
 * `rejectReason`, an on-save toast), but not something to hand back to the LSP server that
 * requested the edit. The wire response therefore always carries this generic reason instead of
 * the detailed one, regardless of which failure produced it.
 */
const APPLY_EDIT_FAILURE_REASON_FOR_SERVER = 'edit rejected'

/**
 * Builds the server→client `workspace/applyEdit` handler for one LSP session, scoped to that
 * session's own project root: every operation's target path(s) must resolve under `allowedRoot`
 * or the whole edit is rejected, and a `TextDocumentEdit` is rejected if its `version` no longer
 * matches what `client` has tracked for that document (stale-edit guard). Without the root scope,
 * any session (any language server, for any open project) could use this request to create,
 * rename, delete, or overwrite files belonging to a *different* open project — `resolve_owning_project`
 * on the Rust side only checks that a path falls under *some* open project, not this session's own.
 */
export const createWorkspaceApplyEditHandler = (monaco: Monaco, allowedRoot: string, client: LspClient): ServerRequestHandler => {
    return async (params) => {
        if (!isApplyWorkspaceEditParams(params)) return { applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' }
        const result = await applyWorkspaceEdit(monaco, params.edit, undefined, {
            allowedRoot,
            getDocumentVersion: client.getDocumentVersion,
        })
        return result.applied ? result : { applied: false, failureReason: APPLY_EDIT_FAILURE_REASON_FOR_SERVER }
    }
}

/**
 * Registers a process-wide fallback `workspace/applyEdit` handler (unscoped — no `allowedRoot`),
 * kept only for the rare case a request arrives outside any session's own registration window.
 * Every real LSP session registers its own root-scoped handler instead, via
 * `client.registerRequestHandler` in `lsp-session-registry.ts`'s `createSession`, which always
 * takes precedence over this one (`client.ts`'s `handleServerRequest` checks the instance-level
 * registry first). Must be called once at app bootstrap with the live `Monaco` instance.
 */
export const registerWorkspaceApplyEditHandler = (monaco: Monaco) =>
    registerServerRequestHandler('workspace/applyEdit', async (params) => {
        if (!isApplyWorkspaceEditParams(params)) return { applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' }
        const result = await applyWorkspaceEdit(monaco, params.edit)
        return result.applied ? result : { applied: false, failureReason: APPLY_EDIT_FAILURE_REASON_FOR_SERVER }
    })
