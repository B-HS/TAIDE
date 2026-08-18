import type { ProjectId } from '@shared/api/bindings'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { WorkspaceEdit } from '@shared/lib/lsp/protocol'
import type { ServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'
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
 *
 * `projectId` — this session's own owning project, known by the caller (`lsp-session-registry.ts`
 * creates one session per `(projectId, serverId)` pair) — is threaded into `applyWorkspaceEdit` so a
 * cross-file edit landing on a background (open-but-unattached) model gets its hot-exit mirror
 * write scoped to *this* project rather than whichever project happens to be globally active when
 * the push arrives; see `workspace-edit-applier.ts`'s `mirrorBackgroundModelEdit` doc comment.
 *
 * There is deliberately no process-wide, unscoped fallback for this method — `client.ts`'s
 * `handleServerRequest` falls back to `server-request-handler-registry.ts` only when no
 * instance-level handler is registered, and a rootless fallback there would let any request that
 * outran this registration touch files under *any* open project. `lsp-session-registry.ts`'s
 * `createSession` therefore registers the handler this factory returns on `client` immediately
 * after creating `client` — before `spawnLspSession` even starts the server process — so no
 * inbound `workspace/applyEdit` can ever arrive before it is answered by this root-scoped handler.
 */
export const createWorkspaceApplyEditHandler = (
    monaco: Monaco,
    allowedRoot: string,
    client: LspClient,
    projectId: ProjectId,
): ServerRequestHandler => {
    return async (params) => {
        if (!isApplyWorkspaceEditParams(params)) return { applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' }
        const result = await applyWorkspaceEdit(monaco, params.edit, undefined, {
            allowedRoot,
            getDocumentVersion: client.getDocumentVersion,
            projectId,
        })
        return result.applied ? result : { applied: false, failureReason: APPLY_EDIT_FAILURE_REASON_FOR_SERVER }
    }
}
