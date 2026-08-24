import { TERMINAL_MIRRORED_TOKENS } from '@shared/lib/theme-convert/ansi-palette'
import { COLOR_NAMESPACES, SYNTAX_TOKENS } from '@shared/lib/theme-convert/ui-token-vocabulary'
import type { SyntaxStyle } from '@shared/lib/theme-convert/types'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'

export const validateCompleteness = (colors: Record<string, string>, syntax: Record<string, SyntaxStyle>, terminal: Record<string, string>) => {
    const requiredColorKeys = COLOR_NAMESPACES.flatMap((namespace) => namespace.tokens.map((token) => `${namespace.id}.${token}`))
    const missingColors = requiredColorKeys.filter((key) => !colors[key])
    const missingSyntax = SYNTAX_TOKENS.filter((key) => !syntax[key]?.fg)
    const missingTerminal = [...TERMINAL_ANSI_TOKENS, ...Object.keys(TERMINAL_MIRRORED_TOKENS)].filter((key) => !terminal[key])

    return { missingColors, missingSyntax, missingTerminal }
}
