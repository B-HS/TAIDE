export const IS_MAC = typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('mac')

export const MOD_KEY_LABEL = IS_MAC ? '⌘' : 'Ctrl'
export const ALT_KEY_LABEL = IS_MAC ? '⌥' : 'Alt'
export const SHIFT_KEY_LABEL = IS_MAC ? '⇧' : 'Shift'
