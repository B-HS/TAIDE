import type { CancellationToken, languages } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { LspCommand } from '@shared/lib/lsp/command-relay'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { LspRange } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco, monacoRangeToLsp } from '@shared/lib/lsp/position'

const NOOP_DISPOSABLE = { dispose: () => {} }

export const CODE_LENS_REFRESH_DEBOUNCE_MS = 300

export type LspCodeLens = { range: LspRange; command?: LspCommand; data?: unknown }

const lensDataByInstance = new WeakMap<languages.CodeLens, unknown>()

export const toMonacoCommand = (command: LspCommand): languages.Command | undefined => {
    if (!command.command) return undefined
    return { id: command.command, title: command.title, arguments: command.arguments }
}

export const toMonacoCodeLens = (lens: LspCodeLens): languages.CodeLens => {
    const monacoLens: languages.CodeLens = {
        range: lspRangeToMonaco(lens.range),
        command: lens.command ? toMonacoCommand(lens.command) : undefined,
    }
    if (lens.data !== undefined) lensDataByInstance.set(monacoLens, lens.data)
    return monacoLens
}

const codeLensRefreshListenersByClient = new WeakMap<LspClient, Set<() => void>>()

/**
 * Fires every `registerCodeLens` registration sharing `client`'s debounced refresh — the
 * `semantic-tokens.ts` `triggerSemanticTokensRefresh` precedent (F7#4), replacing the previous
 * process-wide `workspace/codeLens/refresh` handler in `server-request-handler-registry.ts`: that
 * handler fired *every* open session's listeners on a refresh push from any one server, so two
 * unrelated projects' rust-analyzer/vtsls sessions each recomputed lenses whenever the other's
 * server asked for a refresh. Meant to be called from the session-scoped
 * `workspace/codeLens/refresh` handler `lsp-session-registry.ts`'s `createSession` registers once
 * per LSP client via `client.registerRequestHandler` (the `workspace/applyEdit`/
 * `workspace/semanticTokens/refresh` precedent), so a refresh push only recomputes the sessions
 * that actually asked for it.
 */
export const triggerCodeLensRefresh = (client: LspClient) => {
    codeLensRefreshListenersByClient.get(client)?.forEach((listener) => listener())
}

const subscribeCodeLensRefresh = (client: LspClient, listener: () => void) => {
    const listeners = codeLensRefreshListenersByClient.get(client) ?? new Set()
    listeners.add(listener)
    codeLensRefreshListenersByClient.set(client, listeners)
    return () => listeners.delete(listener)
}

/**
 * Registers the LSP CodeLens provider for `languageId`.
 *
 * `isCodeLensEnabled` gates the `settings.editorCodeLensEnabled` toggle. It is read fresh on
 * every `provideCodeLenses` call rather than gating provider *registration* — flipping the
 * setting therefore takes effect on the very next lens computation, with no need to tear down
 * and recreate the monaco provider (registration is one-time per session/language, but the
 * setting can change at any point during a session). It is an injected getter rather than a
 * shared-module mutable flag (the pattern `applyMonacoKeybindingOverrides` uses) so this
 * `shared/lib` module stays independent of *when/whether* some higher layer remembers to push
 * setting changes in — the caller (widgets layer, which owns the settings query cache) supplies
 * a getter that always reflects the live value, no separate sync wiring required.
 */
export const registerCodeLens = (monaco: Monaco, client: LspClient, languageId: string, isCodeLensEnabled: () => boolean = () => true) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.codeLensProvider))) return NOOP_DISPOSABLE

    const supportsResolve = client.supports((capabilities) => capabilities.codeLensProvider?.resolveProvider === true)

    const onDidChangeEmitter = new monaco.Emitter<languages.CodeLensProvider>()

    const provider: languages.CodeLensProvider = {
        onDidChange: onDidChangeEmitter.event,
        provideCodeLenses: async (model, token: CancellationToken) => {
            if (!isCodeLensEnabled()) return { lenses: [] }
            const result = await client.request<LspCodeLens[] | null>('textDocument/codeLens', {
                textDocument: { uri: model.uri.toString() },
            })
            if (token.isCancellationRequested) return { lenses: [] }
            return { lenses: (result ?? []).map(toMonacoCodeLens) }
        },
        resolveCodeLens: supportsResolve
            ? async (_model, codeLens, token: CancellationToken) => {
                  const params: LspCodeLens = { range: monacoRangeToLsp(codeLens.range), data: lensDataByInstance.get(codeLens) }
                  const result = await client.request<LspCodeLens>('codeLens/resolve', params)
                  if (token.isCancellationRequested) return codeLens
                  return { ...codeLens, command: result.command ? toMonacoCommand(result.command) : undefined }
              }
            : undefined,
    }

    let refreshTimer: ReturnType<typeof setTimeout> | null = null
    const scheduleRefresh = () => {
        if (refreshTimer !== null) clearTimeout(refreshTimer)
        refreshTimer = setTimeout(() => {
            refreshTimer = null
            onDidChangeEmitter.fire(provider)
        }, CODE_LENS_REFRESH_DEBOUNCE_MS)
    }
    const unsubscribeRefresh = subscribeCodeLensRefresh(client, scheduleRefresh)

    const registration = monaco.languages.registerCodeLensProvider(languageId, provider)

    return {
        dispose: () => {
            registration.dispose()
            unsubscribeRefresh()
            if (refreshTimer !== null) clearTimeout(refreshTimer)
            onDidChangeEmitter.dispose()
        },
    }
}
