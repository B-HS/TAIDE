import type { KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { buildMonacoKeybindingOverrideRules } from '@shared/lib/monaco/monaco-keybinding'
import { monaco } from '@shared/lib/monaco/setup'

let activeOverrideDisposable: monaco.IDisposable | null = null

/**
 * Applies every stored `monaco.*` keybinding override to the live monaco instance via
 * `addKeybindingRules`. Always disposes the previous batch first (single-owner rule set —
 * calling this again fully replaces the last application, matching `settings.keymapOverrides`).
 * Rule construction (unbind + rebind + held-modifier widget-navigation companions) lives in
 * `buildMonacoKeybindingOverrideRules`, which is pure and unit-tested.
 */
export const applyMonacoKeybindingOverrides = (overrides: KeymapOverrideEntry[]) => {
    activeOverrideDisposable?.dispose()
    activeOverrideDisposable = null

    const rules = buildMonacoKeybindingOverrideRules(overrides)
    if (rules.length === 0) return
    activeOverrideDisposable = monaco.editor.addKeybindingRules(rules)
}
