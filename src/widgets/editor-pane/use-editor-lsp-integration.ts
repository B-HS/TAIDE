import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { useTranslation } from 'react-i18next'
import type { FileSizeTier, LspServerId, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { getStoredDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'
import { applyCodeActionOrCommand, requestCodeActionsForKind, supportsCodeActionResolve } from '@shared/lib/lsp/adapters/code-action'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import { filterAvailableLspServers } from '@entities/lsp/lsp.constant'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { isLspAttachableTier, useLspSession } from '@widgets/editor-pane/use-lsp-session'
import { peekLspSessionForRoot, waitForLspSessionForRoot } from '@entities/lsp/lsp-session-registry'
import { canRenderCodeEditor } from '@widgets/editor-pane/code-editor-visibility'

/** LSP `CodeActionKind`s the two on-save booleans (`settings.fixAllOnSave`/`organizeImportsOnSave`) map to. */
const CODE_ACTION_KIND_FIX_ALL = 'source.fixAll'
const CODE_ACTION_KIND_ORGANIZE_IMPORTS = 'source.organizeImports'

/**
 * Upper bound on how long Code Actions on Save may block the save. VS Code has no hard timeout
 * here either (only a cancellable progress notification) — this project's decision (contract
 * §3.2) is a hard cap: past it, skip waiting further (already-applied edits stay; anything still
 * in flight lands whenever it lands) and warn instead of stalling an explicit ⌘S indefinitely.
 */
const CODE_ACTIONS_ON_SAVE_TIMEOUT_MS = 5_000

type ResolveLspRootFn = (input: { serverId: LspServerId; filePath: string }) => Promise<string | null>

/**
 * The same root decision `use-lsp-session.ts`'s `attachLspSession` makes when it actually acquires a
 * session (`resolvedRoot ?? projectRoot`, a `resolveLspRoot` rejection swallowed to `null`) — reused
 * here rather than invented fresh, so save-time notify/Code Actions always resolve the *same* root a
 * session for `path` would have been (or will be) acquired under. Exported (and `resolveRoot` taken
 * as a parameter instead of this module importing `resolveLspRoot` itself) purely so
 * `use-editor-lsp-integration.test.ts` can exercise the decision without stubbing the Tauri-backed
 * `@entities/lsp/lsp.ipc` module.
 */
export const resolveLspSessionRootForSave = async (input: {
    serverId: LspServerId
    path: string
    projectRoot: string | null
    resolveRoot: ResolveLspRootFn
}): Promise<string | null> => {
    if (!input.projectRoot) return null
    const resolvedRoot = await input.resolveRoot({ serverId: input.serverId, filePath: input.path }).catch(() => null)
    return resolvedRoot ?? input.projectRoot
}

type UseEditorLspIntegrationInput = {
    projectId: ProjectId
    path: string
    languageId: string | null
    tier: FileSizeTier | null
    editor: monaco.editor.IStandaloneCodeEditor | null
    fixAllOnSave: boolean | undefined
    organizeImportsOnSave: boolean | undefined
    isPending: boolean
    isError: boolean
    isOutsideProjectRoot: boolean
    t: ReturnType<typeof useTranslation>['t']
}

/**
 * Owns `EditorPane`'s LSP session attachment (`useLspSession`) and the two save-time LSP
 * integrations: notifying already-attached sessions of a completed disk write
 * (`textDocument/didSave`), and running Code Actions on Save (`source.fixAll` /
 * `source.organizeImports`) before the buffer is written. `isOutsideProjectRoot` (a CLI-opened
 * file such as Claude Code's Ctrl+G temp file — see `editor-pane.tsx`) turns all three off: no
 * session is attached, so neither save-time integration has anything to talk to.
 */
export const useEditorLspIntegration = ({
    projectId,
    path,
    languageId,
    tier,
    editor,
    fixAllOnSave,
    organizeImportsOnSave,
    isPending,
    isError,
    isOutsideProjectRoot,
    t,
}: UseEditorLspIntegrationInput) => {
    const { data: lspServers } = useQuery(lspServersQueryOptions())
    const { data: project } = useQuery(projectQueryOptions(projectId))

    /**
     * LSP servers attached for `languageId` — must stay in exact lockstep with `use-lsp-session.ts`'s
     * own attach gate (language-matching, installed/available, `isLspAttachableTier`, *and* the
     * outside-root exclusion), not just approximate it: a serverId this returns but `use-lsp-session`
     * would never actually attach a session for (e.g. a large/read-only-tier file) makes
     * `waitForLspSessionForRoot` below wait on a session that will never be created.
     */
    const matchingLspServerIds = (forLanguageId: string, forTier: FileSizeTier | null) =>
        !isOutsideProjectRoot && isLspAttachableTier(forTier)
            ? filterAvailableLspServers(lspServers ?? [], forLanguageId).map((server) => server.id)
            : []

    const resolveRootForServer = (serverId: LspServerId) =>
        resolveLspSessionRootForSave({ serverId, path, projectRoot: project?.root ?? null, resolveRoot: resolveLspRoot })

    /**
     * Sends `textDocument/didSave` to every already-attached session for this file once the disk
     * write succeeds — LSP servers that key diagnostics/state off save (not just in-memory
     * `didChange`) need this notification to fire at all. Uses `peekLspSessionForRoot` (no waiting)
     * rather than `waitForLspSessionForRoot`: there is nothing to notify a session that was never
     * attached in the first place, and waiting for one to *become* attached here would just leak an
     * unresolved waiter for every save on a file with no matching session. Root-exact (contract
     * root-aware batch 4) — a project with more than one root open for a `shares_sessions: false`
     * server otherwise risked `textDocument/didSave` landing on whichever root's session happened to
     * be the oldest-inserted one (`peekLspSession`'s root-agnostic lookup), not the one that actually
     * has `path` open.
     */
    const notifyLspSessionsOfSave = async () => {
        if (!languageId) return
        const uri = monaco.Uri.file(path).toString()

        await Promise.all(
            matchingLspServerIds(languageId, tier).map(async (serverId) => {
                const root = await resolveRootForServer(serverId)
                if (!root) return
                const session = peekLspSessionForRoot(projectId, serverId, root)
                const ready = await session?.ready.catch(() => null)
                ready?.client.didSave(uri)
            }),
        )
    }

    /**
     * Runs `source.fixAll` then `source.organizeImports` (fixAll first, matching VS Code's own
     * save-participant ordering) against every attached, code-action-capable LSP session for this
     * file, applying whatever each returns via {@link applyCodeActionOrCommand}. Bypasses monaco's
     * `CodeActionController` entirely — it drives the client directly, the same way the outline
     * panel does for `textDocument/documentSymbol` — because `editor.action.organizeImports`'s
     * `run()` does not await the edit being applied (confirmed against monaco's source), so it
     * cannot be used to sequence code-action → format → save deterministically. Bounded by
     * {@link CODE_ACTIONS_ON_SAVE_TIMEOUT_MS}; a timeout stops *waiting* (format/save proceed with
     * whatever already landed) rather than aborting the underlying LSP requests. Root-exact (contract
     * root-aware batch 4) — see {@link notifyLspSessionsOfSave}'s doc for why root-agnostic lookup is
     * wrong in a multi-root project.
     */
    const runCodeActionsOnSave = async () => {
        const model = editor?.getModel()
        if (!languageId || !model) return

        const kinds = [fixAllOnSave && CODE_ACTION_KIND_FIX_ALL, organizeImportsOnSave && CODE_ACTION_KIND_ORGANIZE_IMPORTS].filter(
            (kind): kind is string => Boolean(kind),
        )
        if (kinds.length === 0) return

        const serverIds = matchingLspServerIds(languageId, tier)
        if (serverIds.length === 0) return

        const uri = model.uri.toString()
        const range = monacoRangeToLsp(model.getFullModelRange())

        /**
         * Cancel handles for every `waitForLspSessionForRoot` waiter registered below, invoked only
         * if the overall timeout fires — a waiter left pending past that point (e.g. a session whose
         * spawn/initialize never settles) would otherwise sit in `waitersByKey` forever, growing by
         * one every time this runs. On the normal (non-timeout) path each waiter already resolves
         * and self-removes, so there is nothing to clean up there.
         */
        const pendingCancels: (() => void)[] = []
        const applyAllKinds = (async () => {
            for (const serverId of serverIds) {
                const root = await resolveRootForServer(serverId)
                if (!root) continue
                const waiter = waitForLspSessionForRoot(projectId, serverId, root)
                pendingCancels.push(waiter.cancel)
                const session = await waiter.promise
                const ready = await session?.ready.catch(() => null)
                if (!ready) continue

                const supportsResolve = supportsCodeActionResolve(ready.client)
                const diagnostics = getStoredDiagnostics(serverId, uri, ready.client)
                for (const kind of kinds) {
                    const actions = await requestCodeActionsForKind(ready.client, uri, range, diagnostics, kind).catch(() => [])
                    for (const action of actions) await applyCodeActionOrCommand(monaco, ready.client, supportsResolve, action).catch(() => undefined)
                }
            }
        })()

        let timeoutId: ReturnType<typeof setTimeout> | undefined
        const timedOut = await Promise.race([
            applyAllKinds.then(() => false),
            new Promise<boolean>((resolve) => (timeoutId = setTimeout(() => resolve(true), CODE_ACTIONS_ON_SAVE_TIMEOUT_MS))),
        ])
        clearTimeout(timeoutId)
        if (timedOut) {
            pendingCancels.forEach((cancel) => cancel())
            toast.error(t('editor.codeActionsOnSaveSkipped'))
        }
    }

    useLspSession({
        projectId,
        path,
        languageId,
        tier,
        enabled: !isOutsideProjectRoot && canRenderCodeEditor(isPending, isError, tier),
    })

    return { notifyLspSessionsOfSave, runCodeActionsOnSave }
}
