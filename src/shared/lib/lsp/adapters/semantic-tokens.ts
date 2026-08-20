import type { CancellationToken, languages } from 'monaco-editor'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { NOOP_DISPOSABLE } from '@shared/lib/lsp/noop-disposable'
import type { SemanticTokens, SemanticTokensDelta, SemanticTokensEdit, SemanticTokensLegend } from '@shared/lib/lsp/protocol'
import { isCapabilityEnabled, SEMANTIC_TOKEN_MODIFIERS } from '@shared/lib/lsp/protocol'
import { lookupSemanticTokenTypeMapping, SYNTAX_TOKENS, toSemanticTokenLegendScope } from '@shared/lib/theme-convert/mapping-tables'

/** LSP encodes each semantic token as a 5-number tuple: `[deltaLine, deltaStart, length, tokenType, tokenModifiers]`. */
const SEMANTIC_TOKEN_TUPLE_SIZE = 5

const emptySemanticTokens = (): languages.SemanticTokens => ({ resultId: undefined, data: new Uint32Array(0) })

const isSemanticTokensDelta = (value: SemanticTokens | SemanticTokensDelta): value is SemanticTokensDelta => 'edits' in value

export type DecodedSemanticToken = { line: number; char: number; length: number; typeIndex: number; modifierBitmask: number }

/** Decodes LSP's line/char-relative 5-tuple stream into absolute-position tokens, in stream order. */
export const decodeSemanticTokensData = (data: readonly number[]): DecodedSemanticToken[] => {
    const tokens: DecodedSemanticToken[] = []
    let line = 0
    let char = 0
    for (let index = 0; index < data.length; index += SEMANTIC_TOKEN_TUPLE_SIZE) {
        const deltaLine = data[index]
        const deltaStart = data[index + 1]
        line += deltaLine
        char = deltaLine === 0 ? char + deltaStart : deltaStart
        tokens.push({ line, char, length: data[index + 2], typeIndex: data[index + 3], modifierBitmask: data[index + 4] })
    }
    return tokens
}

const remapModifierBitmask = (bitmask: number, modifierIndexByServerIndex: readonly (number | undefined)[]) =>
    modifierIndexByServerIndex.reduce<number>((result, mappedIndex, bit) => {
        if (mappedIndex === undefined || (bitmask & (1 << bit)) === 0) return result
        return result | (1 << mappedIndex)
    }, 0)

/**
 * Re-encodes decoded tokens back into LSP's relative 5-tuple stream, keeping only tokens whose
 * type survived {@link buildSemanticTokensLegendMapping}'s mapping. Deltas are recomputed against
 * the previous *kept* token (not the previous token in the original stream), so a dropped token's
 * position offset is folded into the next surviving token's delta instead of corrupting it.
 */
export const reencodeSemanticTokens = (
    decoded: readonly DecodedSemanticToken[],
    typeIndexByServerIndex: readonly (number | undefined)[],
    modifierIndexByServerIndex: readonly (number | undefined)[],
): Uint32Array => {
    const encoded: number[] = []
    let prevLine = 0
    let prevChar = 0
    for (const decodedToken of decoded) {
        const mappedTypeIndex = typeIndexByServerIndex[decodedToken.typeIndex]
        if (mappedTypeIndex === undefined) continue
        const deltaLine = decodedToken.line - prevLine
        const deltaStart = deltaLine === 0 ? decodedToken.char - prevChar : decodedToken.char
        encoded.push(
            deltaLine,
            deltaStart,
            decodedToken.length,
            mappedTypeIndex,
            remapModifierBitmask(decodedToken.modifierBitmask, modifierIndexByServerIndex),
        )
        prevLine = decodedToken.line
        prevChar = decodedToken.char
    }
    return new Uint32Array(encoded)
}

/**
 * Applies a `textDocument/semanticTokens/full/delta` response's splice edits to the previously
 * cached raw server `data` array. Every `edit.start`/`deleteCount` pair is an offset into the
 * *original* (pre-edit) `base` array — the same contract monaco's own reference merge
 * (`documentSemanticTokens.js`'s `_setDocumentSemanticTokens`) implements by walking `edits` from
 * last to first. Applying edits in stream (ascending) order instead, each against the
 * *already-spliced* result of the previous one, is a different and incorrect semantics: once an
 * earlier edit shifts the array, a later edit's `start` no longer points at the original position
 * the server meant. Processing in reverse sidesteps that without needing monaco's explicit
 * destination-buffer bookkeeping: edits are non-overlapping and sorted ascending per the LSP spec,
 * so splicing the highest-`start` edit first never disturbs the still-original indices any
 * lower-`start` edit (processed next) still needs.
 */
export const applySemanticTokensDeltaEdits = (base: readonly number[], edits: readonly SemanticTokensEdit[]) =>
    [...edits].reverse().reduce((current, edit) => current.toSpliced(edit.start, edit.deleteCount, ...(edit.data ?? [])), [...base])

export type SemanticTokensLegendMapping = {
    legend: SemanticTokensLegend
    typeIndexByServerIndex: (number | undefined)[]
    modifierIndexByServerIndex: (number | undefined)[]
}

/**
 * Builds the adapter's own monaco-facing legend from the server's declared legend, plus the
 * index-remap tables {@link reencodeSemanticTokens} needs. Type names absent from
 * `SEMANTIC_TOKEN_TYPE_MAP` map to `undefined` (dropped downstream, via
 * {@link lookupSemanticTokenTypeMapping} so a legend type name that happens to collide with an
 * `Object.prototype` member like `constructor` drops too, instead of resolving to that member). The
 * output legend's `tokenTypes` is `SYNTAX_TOKENS`-ordered (not server-order) so it is stable and
 * independent of how any given server happened to declare its own legend, and every entry is run
 * through {@link toSemanticTokenLegendScope} — never a bare `SYNTAX_TOKENS` name — so the type
 * string monaco is told to look up can never exact-match a real TextMate scope (`build-shiki-theme.ts`'s
 * `buildSemanticTokenThemeRules` scopes its theme rules under that same namespaced string). Modifiers
 * always pass through as the standard 10 names (`SEMANTIC_TOKEN_MODIFIERS`) regardless of what the
 * server declared — a non-standard modifier bit is dropped, not renamed.
 */
export const buildSemanticTokensLegendMapping = (serverLegend: SemanticTokensLegend): SemanticTokensLegendMapping => {
    const mappedTypes = serverLegend.tokenTypes.map(lookupSemanticTokenTypeMapping)
    const usedTypes = new Set(mappedTypes.filter((mapped): mapped is (typeof SYNTAX_TOKENS)[number] => mapped !== undefined))
    const syntaxTokenTypes: (typeof SYNTAX_TOKENS)[number][] = SYNTAX_TOKENS.filter((token) => usedTypes.has(token))
    const tokenTypes: string[] = syntaxTokenTypes.map(toSemanticTokenLegendScope)
    const typeIndexByServerIndex = mappedTypes.map((mapped) => (mapped === undefined ? undefined : syntaxTokenTypes.indexOf(mapped)))

    const tokenModifiers: string[] = [...SEMANTIC_TOKEN_MODIFIERS]
    const modifierIndexByServerIndex = serverLegend.tokenModifiers.map((modifierName) => {
        const index = tokenModifiers.indexOf(modifierName)
        return index === -1 ? undefined : index
    })

    return { legend: { tokenTypes, tokenModifiers }, typeIndexByServerIndex, modifierIndexByServerIndex }
}

type SemanticTokensOutcome = { resultId: string | undefined; serverData: number[] }

const requestFullSemanticTokens = async (client: LspClient, uri: string, token: CancellationToken): Promise<SemanticTokensOutcome | null> => {
    const full = await client.request<SemanticTokens | null>('textDocument/semanticTokens/full', { textDocument: { uri } })
    if (token.isCancellationRequested || !full) return null
    return { resultId: full.resultId, serverData: full.data }
}

const requestDeltaSemanticTokens = async (
    client: LspClient,
    uri: string,
    previousResultId: string,
    baseServerData: readonly number[],
    token: CancellationToken,
): Promise<SemanticTokensOutcome | null> => {
    const delta = await client.request<SemanticTokens | SemanticTokensDelta | null>('textDocument/semanticTokens/full/delta', {
        textDocument: { uri },
        previousResultId,
    })
    if (token.isCancellationRequested || !delta) return null
    return isSemanticTokensDelta(delta)
        ? { resultId: delta.resultId, serverData: applySemanticTokensDeltaEdits(baseServerData, delta.edits) }
        : { resultId: delta.resultId, serverData: delta.data }
}

const refreshListenersByClient = new WeakMap<LspClient, Set<() => void>>()

/**
 * Fires every `registerSemanticTokens` registration sharing `client`'s `onDidChange` emitter.
 * Meant to be called from the session-scoped `workspace/semanticTokens/refresh` handler Phase D
 * registers once per LSP client via `client.registerRequestHandler` (the `workspace/applyEdit`
 * precedent — see `lsp-session-registry.ts`'s `buildInitializeParams` doc comment), so a refresh
 * push only recomputes the sessions that actually asked for it (no cross-session storm). One
 * client can back several registrations at once when it serves multiple language ids from a single
 * process (e.g. vtsls across javascript/typescript/jsx/tsx).
 */
export const triggerSemanticTokensRefresh = (client: LspClient) => {
    refreshListenersByClient.get(client)?.forEach((listener) => listener())
}

const subscribeSemanticTokensRefresh = (client: LspClient, listener: () => void) => {
    const listeners = refreshListenersByClient.get(client) ?? new Set()
    listeners.add(listener)
    refreshListenersByClient.set(client, listeners)
    return () => listeners.delete(listener)
}

/**
 * Registers the LSP semantic tokens provider for `languageId`. `isEnabled` gates
 * `settings.editorSemanticHighlighting` the same way `registerCodeLens`'s `isCodeLensEnabled` gates
 * its own setting — read fresh per request, no re-registration on toggle.
 *
 * Always answers with a full `SemanticTokens` object, never `SemanticTokensEdits`: a
 * `textDocument/semanticTokens/full/delta` response (when the server supports it and the model's
 * cached `resultId` still matches monaco's `lastResultId`) is applied against the cached raw server
 * `data` and the *entire* result is re-encoded, because re-encoding must run over the full token
 * set regardless (dropped/unmapped types shift every downstream delta) — delta's payoff is
 * realized server-side (less recomputation) and on the wire (smaller response), not in what this
 * adapter hands monaco.
 */
export const registerSemanticTokens = (monaco: Monaco, client: LspClient, languageId: string, isEnabled: () => boolean = () => true) => {
    if (!client.supports((capabilities) => isCapabilityEnabled(capabilities.semanticTokensProvider?.full))) return NOOP_DISPOSABLE

    const serverLegend = client.getCapabilities()?.semanticTokensProvider?.legend
    if (!serverLegend) return NOOP_DISPOSABLE

    const deltaSupported = client.supports(
        (capabilities) =>
            capabilities.semanticTokensProvider !== undefined &&
            typeof capabilities.semanticTokensProvider.full === 'object' &&
            capabilities.semanticTokensProvider.full.delta === true,
    )

    const { legend, typeIndexByServerIndex, modifierIndexByServerIndex } = buildSemanticTokensLegendMapping(serverLegend)

    const cacheByModelUri = new Map<string, { resultId: string; serverData: number[] }>()

    const onDidChangeEmitter = new monaco.Emitter<void>()

    const provider: languages.DocumentSemanticTokensProvider = {
        onDidChange: onDidChangeEmitter.event,
        getLegend: () => legend,
        provideDocumentSemanticTokens: async (model, lastResultId, token: CancellationToken) => {
            const uri = model.uri.toString()
            if (!isEnabled()) {
                cacheByModelUri.delete(uri)
                return emptySemanticTokens()
            }

            const cached = cacheByModelUri.get(uri)
            const outcome =
                deltaSupported && cached !== undefined && lastResultId !== null && cached.resultId === lastResultId
                    ? await requestDeltaSemanticTokens(client, uri, cached.resultId, cached.serverData, token)
                    : await requestFullSemanticTokens(client, uri, token)
            if (!outcome) return emptySemanticTokens()

            if (outcome.resultId !== undefined) cacheByModelUri.set(uri, { resultId: outcome.resultId, serverData: outcome.serverData })
            else cacheByModelUri.delete(uri)

            const decoded = decodeSemanticTokensData(outcome.serverData)
            return { resultId: outcome.resultId, data: reencodeSemanticTokens(decoded, typeIndexByServerIndex, modifierIndexByServerIndex) }
        },
        releaseDocumentSemanticTokens: (resultId) => {
            if (resultId === undefined) return
            for (const [uri, entry] of cacheByModelUri) {
                if (entry.resultId !== resultId) continue
                cacheByModelUri.delete(uri)
                break
            }
        },
    }

    const registration = monaco.languages.registerDocumentSemanticTokensProvider(languageId, provider)
    const unsubscribeRefresh = subscribeSemanticTokensRefresh(client, () => onDidChangeEmitter.fire())

    return {
        dispose: () => {
            registration.dispose()
            unsubscribeRefresh()
            cacheByModelUri.clear()
            onDidChangeEmitter.dispose()
        },
    }
}
