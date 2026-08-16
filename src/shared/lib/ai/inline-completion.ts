import type { AiInlineCompleteRequest, AiInlineCompleteResponse, AiProviderId, AiTokenStatus, Settings } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

export const AUTO_TAB_DEBOUNCE_MS = 300

const INLINE_COMPLETION_PREFIX_MAX_CHARS = 4_000
const INLINE_COMPLETION_SUFFIX_MAX_CHARS = 2_000
const INLINE_COMPLETION_CACHE_MAX_ENTRIES = 50
const AI_INLINE_COMPLETION_GROUP_ID = 'taide.ai-inline-completion'

export type AiInlineCompletionConfig = {
    provider: AiProviderId
    model: string
}

/**
 * IPC boundary the provider calls through — injected by the caller (`ai_inline_complete`/
 * `ai_request_cancel` from `entities/ai/ai.ipc`) so this `shared` module never imports `entities`
 * directly (fsd.md §2), mirroring how `shared/lib/lsp/adapters/*` take an injected `LspClient`.
 */
export type AiInlineCompletionClient = {
    complete: (request: AiInlineCompleteRequest) => Promise<AiInlineCompleteResponse>
    cancel: (requestId: string) => Promise<unknown>
}

type GetAiInlineCompletionConfig = () => AiInlineCompletionConfig | null

/**
 * Derives the active auto-tab provider/model pair from settings, or null when auto-tab has no
 * usable configuration (no provider/model chosen yet, or the chosen provider's token was cleared).
 */
export const resolveAiInlineCompletionConfig = (
    settings: Pick<Settings, 'aiProvider' | 'aiModel'> | undefined,
    tokenStatus: AiTokenStatus | undefined,
): AiInlineCompletionConfig | null => {
    const provider = settings?.aiProvider as AiProviderId | undefined
    const model = settings?.aiModel
    if (!provider || !model) return null
    if (!(tokenStatus?.[provider] ?? false)) return null
    return { provider, model }
}

/**
 * djb2 — fast, non-cryptographic string hash used only to build a cache key.
 */
const hashText = (text: string) => {
    let hash = 5381
    for (let index = 0; index < text.length; index += 1) hash = (hash * 33) ^ text.charCodeAt(index)
    return (hash >>> 0).toString(36)
}

const buildCacheKey = (filePath: string, config: AiInlineCompletionConfig, prefix: string, suffix: string) =>
    `${filePath}::${config.provider}::${config.model}::${hashText(prefix)}::${hashText(suffix)}`

const completionCache = new Map<string, string>()

const readCache = (key: string) => completionCache.get(key)

const writeCache = (key: string, text: string) => {
    if (completionCache.size >= INLINE_COMPLETION_CACHE_MAX_ENTRIES) {
        const oldestKey = completionCache.keys().next().value
        if (oldestKey !== undefined) completionCache.delete(oldestKey)
    }
    completionCache.set(key, text)
}

const buildContextWindow = (model: monaco.editor.ITextModel, position: monaco.Position) => {
    const cursorOffset = model.getOffsetAt(position)
    const prefixStart = model.getPositionAt(Math.max(0, cursorOffset - INLINE_COMPLETION_PREFIX_MAX_CHARS))
    const suffixEnd = model.getPositionAt(Math.min(model.getValueLength(), cursorOffset + INLINE_COMPLETION_SUFFIX_MAX_CHARS))

    return {
        prefix: model.getValueInRange({
            startLineNumber: prefixStart.lineNumber,
            startColumn: prefixStart.column,
            endLineNumber: position.lineNumber,
            endColumn: position.column,
        }),
        suffix: model.getValueInRange({
            startLineNumber: position.lineNumber,
            startColumn: position.column,
            endLineNumber: suffixEnd.lineNumber,
            endColumn: suffixEnd.column,
        }),
    }
}

const waitForDebounce = (token: monaco.CancellationToken, delayMs: number) =>
    new Promise<boolean>((resolve) => {
        const timer = setTimeout(() => {
            listener.dispose()
            resolve(false)
        }, delayMs)
        const listener = token.onCancellationRequested(() => {
            clearTimeout(timer)
            resolve(true)
        })
    })

const toInlineCompletions = (text: string, position: monaco.Position): monaco.languages.InlineCompletions => ({
    items: [
        {
            insertText: text,
            range: {
                startLineNumber: position.lineNumber,
                startColumn: position.column,
                endLineNumber: position.lineNumber,
                endColumn: position.column,
            },
        },
    ],
    enableForwardStability: true,
})

const createInlineCompletionsProvider = (
    monacoInstance: typeof monaco,
    getConfig: GetAiInlineCompletionConfig,
    getClient: () => AiInlineCompletionClient,
): monaco.languages.InlineCompletionsProvider => ({
    groupId: AI_INLINE_COMPLETION_GROUP_ID,
    provideInlineCompletions: async (model, position, context, token) => {
        const config = getConfig()
        if (!config) return null

        const filePath = model.uri.path
        const { prefix, suffix } = buildContextWindow(model, position)
        const cacheKey = buildCacheKey(filePath, config, prefix, suffix)
        const cached = readCache(cacheKey)
        if (cached !== undefined) return toInlineCompletions(cached, position)

        const isAutomatic = context.triggerKind === monacoInstance.languages.InlineCompletionTriggerKind.Automatic
        if (isAutomatic && (await waitForDebounce(token, AUTO_TAB_DEBOUNCE_MS))) return null
        if (token.isCancellationRequested) return null

        const client = getClient()
        const requestId = crypto.randomUUID()
        const cancelListener = token.onCancellationRequested(() => void client.cancel(requestId).catch(() => undefined))

        try {
            const response = await client.complete({
                requestId,
                provider: config.provider,
                model: config.model,
                prefix,
                suffix,
                language: model.getLanguageId(),
                filePath,
            })
            if (token.isCancellationRequested || !response.text) return null

            writeCache(cacheKey, response.text)
            return toInlineCompletions(response.text, position)
        } catch {
            return null
        } finally {
            cancelListener.dispose()
        }
    },
    disposeInlineCompletions: () => {},
})

type AiInlineCompletionOwner = { getConfig: GetAiInlineCompletionConfig; client: AiInlineCompletionClient }

let sharedProviderDisposable: monaco.IDisposable | null = null
/**
 * Every mounted `CodeEditor` that has auto-tab enabled pushes an entry here (one per pane, since
 * `pane-node-view.tsx` mounts multiple `CodeEditor`s for split layouts). The registered Monaco
 * provider always reads the last (most recently acquired, still-live) entry — so when a pane
 * unmounts and removes its own entry, requests naturally fall back to whichever pane acquired
 * before it, instead of keeping a single global ref that can point at an already-unmounted pane.
 */
const owners: AiInlineCompletionOwner[] = []

export const acquireAiInlineCompletionProvider = (
    monacoInstance: typeof monaco,
    getConfig: GetAiInlineCompletionConfig,
    client: AiInlineCompletionClient,
) => {
    const owner: AiInlineCompletionOwner = { getConfig, client }
    owners.push(owner)
    if (!sharedProviderDisposable)
        sharedProviderDisposable = monacoInstance.languages.registerInlineCompletionsProvider(
            '*',
            createInlineCompletionsProvider(
                monacoInstance,
                () => owners.at(-1)?.getConfig() ?? null,
                () => {
                    const activeOwner = owners.at(-1)
                    if (!activeOwner) throw new Error('AiInlineCompletionClient is not registered')
                    return activeOwner.client
                },
            ),
        )

    return () => {
        const index = owners.indexOf(owner)
        if (index !== -1) owners.splice(index, 1)
        if (owners.length > 0) return
        sharedProviderDisposable?.dispose()
        sharedProviderDisposable = null
    }
}
