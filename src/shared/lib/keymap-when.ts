/**
 * Import verified DOM-free-safe by direct `bun:test` load (Wave H contract §3.2 load gate): the
 * module's dependency chain (`platform.js`, `nls.js`, `scanner.js`, `instantiation.js`) only reads
 * `window`/`document` behind `typeof` guards, unlike `standaloneServices.js` (the reason
 * `command-relay.ts` deep-imports dynamically). A static top-level import is used here — not the
 * dynamic-import precedent that file set — because `when` must be evaluated *synchronously* inside
 * the keydown handler (there is no point at which awaiting a dynamic import and then calling
 * `preventDefault` afterwards would still be able to prevent the browser's default action). Full
 * record: `docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md`.
 */
import { ContextKeyExpr } from 'monaco-editor/platform/contextkey/common/contextkey'
import type { ContextKeyExpression } from 'monaco-editor/platform/contextkey/common/contextkey'
import type { KeymapContextGetters } from '@shared/lib/keymap-context'
import { DEFAULT_KEYMAP_CONTEXT_GETTERS } from '@shared/lib/keymap-context'

const deserializedExpressionCache = new Map<string, ContextKeyExpression | undefined>()

const deserializeKeymapWhen = (when: string) => {
    if (deserializedExpressionCache.has(when)) return deserializedExpressionCache.get(when)
    const expression = ContextKeyExpr.deserialize(when)
    deserializedExpressionCache.set(when, expression)
    return expression
}

/**
 * Evaluates a `KeymapEntry.when` clause against the live pull-context. `when === undefined` (most
 * entries) is always satisfied. An unparseable clause disables the entry outright (`false`) rather
 * than falling back to "always match" — a silently-broken `when` string should never make a
 * context-scoped binding fire everywhere unconditionally.
 */
export const evaluateKeymapWhen = (when: string | undefined, getters: KeymapContextGetters = DEFAULT_KEYMAP_CONTEXT_GETTERS) => {
    if (when === undefined) return true
    const expression = deserializeKeymapWhen(when)
    if (!expression) return false
    return expression.evaluate({ getValue: (key) => getters[key]?.() ?? false })
}
