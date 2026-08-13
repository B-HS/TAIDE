import type { KeymapModifier } from '@shared/lib/keymap'

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

export const MONACO_ACTION_ID_PREFIX = 'monaco.'

export const isMonacoCommandId = (id: string) => id.startsWith(MONACO_ACTION_ID_PREFIX)

export const toMonacoActionId = (commandOrOverrideId: string) =>
    isMonacoCommandId(commandOrOverrideId) ? commandOrOverrideId.slice(MONACO_ACTION_ID_PREFIX.length) : commandOrOverrideId
