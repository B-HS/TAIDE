import type { KeymapOverrideEntry } from '@shared/lib/keymap'
import { buildMonacoKeybinding, isMonacoCommandId, toMonacoActionId } from '@shared/lib/monaco-keybinding'
import { monaco } from '@shared/lib/monaco/setup'

let activeOverrideDisposable: monaco.IDisposable | null = null

const buildUnbindRule = (actionId: string): monaco.editor.IKeybindingRule => ({ keybinding: 0, command: `-${actionId}` })

/**
 * Applies every stored `monaco.*` keybinding override to the live monaco instance via
 * `addKeybindingRules`. Always disposes the previous batch first (single-owner rule set —
 * calling this again fully replaces the last application, matching `settings.keymapOverrides`).
 * An override whose key has no KeyCode mapping is skipped entirely — the monaco default stays
 * active rather than being unbound with nothing to replace it.
 */
export const applyMonacoKeybindingOverrides = (overrides: KeymapOverrideEntry[]) => {
    activeOverrideDisposable?.dispose()
    activeOverrideDisposable = null

    const rules = overrides
        .filter((override) => isMonacoCommandId(override.actionId))
        .flatMap((override): monaco.editor.IKeybindingRule[] => {
            const actionId = toMonacoActionId(override.actionId)
            const unbindRule = buildUnbindRule(actionId)
            if (!override.key) return [unbindRule]

            const keybinding = buildMonacoKeybinding(override.key, override.mods)
            if (keybinding === null) return []

            return [unbindRule, { keybinding, command: actionId }]
        })

    if (rules.length === 0) return
    activeOverrideDisposable = monaco.editor.addKeybindingRules(rules)
}
