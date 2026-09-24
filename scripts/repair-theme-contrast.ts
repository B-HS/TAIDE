import { readdir } from 'node:fs/promises'
import { join } from 'node:path'
import { format, resolveConfig } from 'prettier'
import { TERMINAL_MIRRORED_TOKENS } from '@shared/lib/theme-convert/ansi-palette'
import { isExemptComponentContrastViolation, repairComponentContrast, validateComponentContrast } from '@shared/lib/theme-convert/contrast'
import { validateStateDistinctness } from '@shared/lib/theme-convert/state-distinctness'

const BUNDLED_THEMES_DIR = 'crates/taide-theme/resources/themes'
const JSON_EXTENSION = '.json'

/**
 * Prettier's JSON printer preserves whatever object expansion its input already has, so the indent
 * handed to `JSON.stringify` decides whether short objects (`syntax` entries, `tokenColors`
 * settings) stay on one line or are broken apart. The committed catalog uses both styles, so the
 * style is detected per file rather than assumed, otherwise a two-token repair turns into a
 * whole-file rewrite. `4` reproduces the expanded style and `0` the collapsed one. Same reasoning
 * and same constants as `repair-theme-state-distinctness.ts`.
 */
const JSON_INDENT_CANDIDATES = [4, 0]

type BundledTheme = {
    id: string
    colors: Record<string, string>
    terminal: Record<string, string>
}

/**
 * `theme.terminal` is a flattened mirror of the `terminal.*` entries of `theme.colors` (see
 * `TERMINAL_MIRRORED_TOKENS` and `resolve-terminal.ts`), and it is the copy xterm actually reads
 * (`xterm-theme.ts`). A repair that moves `colors['terminal.foreground']` without moving
 * `terminal.foreground` would fix the lint and leave the terminal unchanged.
 */
const mirrorTerminalTokens = (colors: Record<string, string>, terminal: Record<string, string>) => {
    const mirrored = { ...terminal }
    for (const [terminalKey, colorKey] of Object.entries(TERMINAL_MIRRORED_TOKENS)) {
        if (colors[colorKey]) mirrored[terminalKey] = colors[colorKey]
    }
    return mirrored
}

const serializeLikeSource = async (source: string, repaired: BundledTheme, filePath: string) => {
    const prettierConfig = await resolveConfig(filePath)
    const print = (indent: number, theme: BundledTheme) => format(JSON.stringify(theme, null, indent), { ...prettierConfig, filepath: filePath })
    const parsed = JSON.parse(source) as BundledTheme
    for (const indent of JSON_INDENT_CANDIDATES) {
        if ((await print(indent, parsed)) === source) return print(indent, repaired)
    }
    return print(JSON_INDENT_CANDIDATES[0], repaired)
}

/**
 * Rewrites every bundled theme whose UI text cannot be read on the surface it is drawn on, using the
 * same `repairComponentContrast` the conversion pipeline runs so a re-converted theme and a repaired
 * one land on the same values. Idempotent — a second run finds nothing to repair — so it can be
 * re-run after adding themes or changing the pair table, and prints the per-theme token changes that
 * `docs/theme-system.md` §8.6 records. State distinctness is re-validated afterwards because the two
 * lints share a token (`appSidebar.badge` is a state color in `state-distinctness-pairs.ts` and a
 * text color here), so a contrast repair must not be allowed to collapse a state silently.
 */
const main = async () => {
    const fileNames = (await readdir(BUNDLED_THEMES_DIR)).filter((name) => name.endsWith(JSON_EXTENSION)).sort()
    const unrepaired: string[] = []
    let repairedThemeCount = 0
    let repairCount = 0

    for (const fileName of fileNames) {
        const filePath = join(BUNDLED_THEMES_DIR, fileName)
        const source = await Bun.file(filePath).text()
        const theme = JSON.parse(source) as BundledTheme
        const { colors, repairs } = repairComponentContrast(theme.colors)

        if (repairs.length > 0) {
            const repaired = { ...theme, colors, terminal: mirrorTerminalTokens(colors, theme.terminal) }
            await Bun.write(filePath, await serializeLikeSource(source, repaired, filePath))

            repairedThemeCount += 1
            repairCount += repairs.length
            console.log(`repair-theme-contrast: '${theme.id}' (${repairs.length})`)
            for (const repair of repairs) console.log(`  - ${repair}`)
        }

        unrepaired.push(
            ...validateComponentContrast(colors)
                .filter((error) => !isExemptComponentContrastViolation(theme.id, error))
                .map((error) => `'${theme.id}': ${error}`),
        )
        unrepaired.push(...validateStateDistinctness(colors).map((error) => `'${theme.id}': 구별성 회귀 — ${error}`))
    }

    console.log(
        `repair-theme-contrast: repaired ${repairCount} token${repairCount === 1 ? '' : 's'} across ${repairedThemeCount} of ${fileNames.length} bundled themes`,
    )

    if (unrepaired.length > 0) {
        console.error('repair-theme-contrast: pairs left violating after repair:')
        for (const error of unrepaired) console.error(`  - ${error}`)
        process.exit(1)
    }
}

await main()
