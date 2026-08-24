import { TERMINAL_MIRRORED_TOKENS } from '@shared/lib/theme-convert/ansi-palette'
import type { AnsiLookup } from '@shared/lib/theme-convert/types'

export const resolveTerminal = (resolvedColors: Record<string, string>, ansi: AnsiLookup) => {
    const terminal: Record<string, string> = { ...ansi }

    for (const [terminalKey, colorKey] of Object.entries(TERMINAL_MIRRORED_TOKENS)) {
        terminal[terminalKey] = resolvedColors[colorKey]
    }

    return terminal
}
