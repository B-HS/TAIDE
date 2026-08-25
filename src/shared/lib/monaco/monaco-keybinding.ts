import type { KeymapChordStage, KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'

/**
 * KeyMod bit flags (monaco-editor 0.56, `vs/base/common/keybindings.js` `BinaryKeybindingsMask`).
 * Hardcoded rather than imported from `monaco-editor` because that package touches `window`/`self`
 * at module load and cannot be imported outside a browser/webview runtime (breaks `bun test`).
 */
const MONACO_KEY_MOD = {
    CtrlCmd: 2048,
    Shift: 1024,
    Alt: 512,
    WinCtrl: 256,
} as const

/**
 * KeyCode values (monaco-editor 0.56, `vs/base/common/keyCodes.js`). Covers every key reachable
 * through the app's rebind-capture UI (letters, digits, punctuation, navigation, F1-F12).
 */
const MONACO_KEY_CODE: Record<string, number> = {
    backspace: 1,
    tab: 2,
    enter: 3,
    escape: 9,
    ' ': 10,
    space: 10,
    pageup: 11,
    pagedown: 12,
    end: 13,
    home: 14,
    arrowleft: 15,
    arrowup: 16,
    arrowright: 17,
    arrowdown: 18,
    delete: 20,
    '0': 21,
    '1': 22,
    '2': 23,
    '3': 24,
    '4': 25,
    '5': 26,
    '6': 27,
    '7': 28,
    '8': 29,
    '9': 30,
    a: 31,
    b: 32,
    c: 33,
    d: 34,
    e: 35,
    f: 36,
    g: 37,
    h: 38,
    i: 39,
    j: 40,
    k: 41,
    l: 42,
    m: 43,
    n: 44,
    o: 45,
    p: 46,
    q: 47,
    r: 48,
    s: 49,
    t: 50,
    u: 51,
    v: 52,
    w: 53,
    x: 54,
    y: 55,
    z: 56,
    f1: 59,
    f2: 60,
    f3: 61,
    f4: 62,
    f5: 63,
    f6: 64,
    f7: 65,
    f8: 66,
    f9: 67,
    f10: 68,
    f11: 69,
    f12: 70,
    ';': 85,
    '=': 86,
    ',': 87,
    '-': 88,
    '.': 89,
    '/': 90,
    '`': 91,
    '[': 92,
    '\\': 93,
    ']': 94,
    "'": 95,
}

const MONACO_MODIFIER_MOD: Record<KeymapModifier, number> = {
    mod: MONACO_KEY_MOD.CtrlCmd,
    ctrl: MONACO_KEY_MOD.WinCtrl,
    shift: MONACO_KEY_MOD.Shift,
    alt: MONACO_KEY_MOD.Alt,
}

export const resolveMonacoKeyCode = (key: string) => MONACO_KEY_CODE[key.toLowerCase()] ?? null

/**
 * Builds the numeric keybinding value monaco's `addKeybindingRules` expects
 * (`KeyCode | KeyMod.CtrlCmd | ...`), or `null` when the key has no known KeyCode mapping.
 */
export const buildMonacoKeybinding = (key: string, mods: KeymapModifier[]) => {
    const keyCode = resolveMonacoKeyCode(key)
    if (keyCode === null) return null
    return mods.reduce((combo, mod) => combo | MONACO_MODIFIER_MOD[mod], keyCode)
}

/**
 * Builds the numeric keybinding value for a two-stage chord override, matching monaco's own
 * `decodeKeybinding` packing (`vs/base/common/keybindings.js`): the first stage occupies the low
 * 16 bits, the second stage is shifted into the high 16 bits (`(first & 0xffff) | (second << 16)`).
 * Every value `buildMonacoKeybinding` can produce (max modifier-bits + KeyCode sum is well under
 * 0xffff) fits safely in either half. Returns `null` if either stage has no known KeyCode mapping.
 */
export const buildMonacoChordKeybinding = (firstStage: KeymapChordStage, secondStage: KeymapChordStage) => {
    const first = buildMonacoKeybinding(firstStage.key, firstStage.mods)
    const second = buildMonacoKeybinding(secondStage.key, secondStage.mods)
    if (first === null || second === null) return null
    return (first & 0xffff) | ((second & 0xffff) << 16)
}

export const MONACO_ACTION_ID_PREFIX = 'monaco.'

export const isMonacoCommandId = (id: string) => id.startsWith(MONACO_ACTION_ID_PREFIX)

export const toMonacoActionId = (commandOrOverrideId: string) =>
    isMonacoCommandId(commandOrOverrideId) ? commandOrOverrideId.slice(MONACO_ACTION_ID_PREFIX.length) : commandOrOverrideId

/**
 * The first stage of every `monaco.*` override that carries its own `chord` — additional prefixes
 * `decideKeymapDispatch` must arm the monaco-deferral window for, alongside `MONACO_CHORD_PREFIX_KEY`
 * (Cmd/Ctrl+K). Without this, rebinding a monaco action to a chord under a *different* prefix (e.g.
 * Cmd+J Cmd+S) leaves that prefix unobserved: the app never arms the deferral window for it, so its
 * second stage falls straight through to normal single-stage matching and gets stolen by whichever
 * `APP_KEYMAP` entry happens to own that same key — the exact "2단 키를 window capture 가 삼킨다"
 * defect class Wave H's `MONACO_CHORD_PREFIX_KEY` fixes for the Cmd/Ctrl+K case, recurring for any
 * other prefix the user picks. A rebound *first* stage always reaches monaco fine on its own — this
 * only needs to widen deferral-arming, not the matching each entry already gets.
 */
export const deriveMonacoChordPrefixes = (overrides: KeymapOverrideEntry[]): KeymapChordStage[] =>
    overrides
        .filter((override) => isMonacoCommandId(override.actionId) && override.chord && override.key)
        .map((override) => ({ key: override.key, mods: override.mods }))

export type MonacoKeybindingRule = {
    keybinding: number
    command: string
    when?: string
}

const MONACO_UNBIND_KEYBINDING = 0

const SUGGEST_WIDGET_NAV_WHEN = 'suggestWidgetVisible && textInputFocus && (suggestWidgetMultipleSuggestions || !suggestWidgetHasFocusedSuggestion)'
/**
 * Same as {@link SUGGEST_WIDGET_NAV_WHEN}, but yields to an already-visible, multi-signature
 * parameter hints widget. Monaco's own default only binds Alt+Up/Down to parameter hints
 * navigation (see {@link PARAMETER_HINTS_NAV_WHEN}) because plain Up/Down are already claimed by
 * suggest navigation. When a rebind makes suggest navigation itself held-Alt (e.g. triggerSuggest
 * rebound to Alt+Space), Alt+Up/Down would otherwise collide with that escape hatch whenever both
 * widgets are open at once (a common state while typing function arguments), silently disabling
 * signature-hint cycling. This guard is scoped to Up/Down only — parameter hints never claims
 * PageUp/PageDown, so {@link SUGGEST_WIDGET_NAV_WHEN} is used unmodified there.
 */
const SUGGEST_WIDGET_ARROW_NAV_WHEN = `${SUGGEST_WIDGET_NAV_WHEN} && !(parameterHintsVisible && parameterHintsMultipleSignatures)`
/**
 * Mirrors monaco 0.56's own `showPrevParameterHint`/`showNextParameterHint` `when` clause exactly:
 * `kbExpr: EditorContextKeys.focus` (context key `editorFocus`) ANDed with
 * `precondition: Visible && MultipleSignatures` (`contrib/parameterHints/browser/parameterHints.js`).
 */
const PARAMETER_HINTS_NAV_WHEN = 'editorFocus && parameterHintsVisible && parameterHintsMultipleSignatures'

type WidgetNavCompanion = { key: string; command: string; when: string }

const WIDGET_NAV_COMPANIONS_BY_ACTION_ID: Record<string, WidgetNavCompanion[]> = {
    'editor.action.triggerSuggest': [
        { key: 'arrowup', command: 'selectPrevSuggestion', when: SUGGEST_WIDGET_ARROW_NAV_WHEN },
        { key: 'arrowdown', command: 'selectNextSuggestion', when: SUGGEST_WIDGET_ARROW_NAV_WHEN },
        { key: 'pageup', command: 'selectPrevPageSuggestion', when: SUGGEST_WIDGET_NAV_WHEN },
        { key: 'pagedown', command: 'selectNextPageSuggestion', when: SUGGEST_WIDGET_NAV_WHEN },
    ],
    'editor.action.triggerParameterHints': [
        { key: 'arrowup', command: 'showPrevParameterHint', when: PARAMETER_HINTS_NAV_WHEN },
        { key: 'arrowdown', command: 'showNextParameterHint', when: PARAMETER_HINTS_NAV_WHEN },
    ],
}

const buildUnbindRule = (actionId: string): MonacoKeybindingRule => ({ keybinding: MONACO_UNBIND_KEYBINDING, command: `-${actionId}` })

/**
 * Builds widget-navigation rules for the modifier set the user must hold to fire a rebound
 * popup-trigger action. Monaco ships this coverage for its own default triggers only
 * (suggest: WinCtrl+P/N and CtrlCmd+Up/Down secondaries, parameter hints: Alt+Up/Down),
 * so a rebind like Alt+Space leaves held-Alt+ArrowUp resolving to `moveLinesUpAction`
 * instead of widget navigation. The `when` clauses mirror the monaco 0.56 defaults in
 * `contrib/suggest/browser/suggestController.js` / `contrib/parameterHints/browser/parameterHints.js`,
 * so outside the widget the same keys keep their default commands.
 */
const buildHeldModifierNavRules = (actionId: string, mods: KeymapModifier[]) => {
    const companions = WIDGET_NAV_COMPANIONS_BY_ACTION_ID[actionId] ?? []
    if (mods.length === 0) return []
    return companions.flatMap((companion): MonacoKeybindingRule[] => {
        const keybinding = buildMonacoKeybinding(companion.key, mods)
        return keybinding === null ? [] : [{ keybinding, command: companion.command, when: companion.when }]
    })
}

/**
 * A chord override's `keybinding` is the two-stage encoding from `buildMonacoChordKeybinding`
 * rather than a single-stage `buildMonacoKeybinding` value. Held-modifier widget-nav companions
 * (`buildHeldModifierNavRules`) are skipped entirely for chord overrides — that scheme models a
 * modifier held down through a *simultaneous* arrow press, which has no equivalent once the
 * trigger itself is a two-key *sequence*; only `WIDGET_NAV_COMPANIONS_BY_ACTION_ID`'s two popup
 * triggers use companions at all, and neither is expected to be chord-rebound in practice.
 */
const buildMonacoKeybindingRuleGroup = (override: KeymapOverrideEntry) => {
    const actionId = toMonacoActionId(override.actionId)
    const unbindRule = buildUnbindRule(actionId)
    if (!override.key) return { primary: [unbindRule], companions: [] }

    const keybinding = override.chord
        ? buildMonacoChordKeybinding({ key: override.key, mods: override.mods }, override.chord)
        : buildMonacoKeybinding(override.key, override.mods)
    if (keybinding === null) return { primary: [], companions: [] }

    const companions = override.chord ? [] : buildHeldModifierNavRules(actionId, override.mods)
    return { primary: [unbindRule, { keybinding, command: actionId }], companions }
}

/**
 * Builds the full `addKeybindingRules` batch for every stored `monaco.*` keymap override:
 * per override an unbind rule (`keybinding: 0` + `-command`, which removes only that command's
 * default bindings inside monaco's `KeybindingResolver.handleRemovals`), the rebind rule, and
 * held-modifier widget-navigation companions for popup-trigger actions. An override whose key
 * has no KeyCode mapping is skipped entirely — the monaco default stays active rather than
 * being unbound with nothing to replace it.
 *
 * All companion rules are appended after every override's own unbind/rebind rules, regardless of
 * where in `overrides` they were produced. Monaco's `KeybindingResolver` resolves same-keypress
 * conflicts by taking the *last* rule (by array position) whose `when` clause matches — so without
 * this grouping, a companion rule's priority over an unrelated action rebound to the same
 * held-modifier combo would depend on which override happened to be stored first, silently
 * shadowing widget navigation half the time. Placing companions last makes them win whenever
 * their widget-open `when` clause is true, independent of override storage order.
 */
export const buildMonacoKeybindingOverrideRules = (overrides: KeymapOverrideEntry[]) => {
    const groups = overrides.filter((override) => isMonacoCommandId(override.actionId)).map(buildMonacoKeybindingRuleGroup)
    return [...groups.flatMap((group) => group.primary), ...groups.flatMap((group) => group.companions)]
}
