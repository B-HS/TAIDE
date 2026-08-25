import { compositeOverBackground, hexToRgb } from '@shared/lib/color'
import { isDistinctFromBodyForeground, isOpaqueForegroundCandidate } from '@shared/lib/theme-convert/mapping-tables'

const RGB_CHANNEL_MAX = 255
const SRGB_LINEAR_THRESHOLD = 0.03928
const SRGB_LINEAR_DIVISOR = 12.92
const SRGB_GAMMA_OFFSET = 0.055
const SRGB_GAMMA_DIVISOR = 1.055
const SRGB_GAMMA_EXPONENT = 2.4
const LUMINANCE_WEIGHT_R = 0.2126
const LUMINANCE_WEIGHT_G = 0.7152
const LUMINANCE_WEIGHT_B = 0.0722
const CONTRAST_RATIO_OFFSET = 0.05
const MIN_CONTRAST_RATIO = 3

const srgbChannelToLinear = (channel: number) => {
    const normalized = channel / RGB_CHANNEL_MAX
    return normalized <= SRGB_LINEAR_THRESHOLD
        ? normalized / SRGB_LINEAR_DIVISOR
        : ((normalized + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR) ** SRGB_GAMMA_EXPONENT
}

const relativeLuminance = (hex: string): number | null => {
    const rgb = hexToRgb(hex)
    if (!rgb) return null
    return (
        LUMINANCE_WEIGHT_R * srgbChannelToLinear(rgb.r) +
        LUMINANCE_WEIGHT_G * srgbChannelToLinear(rgb.g) +
        LUMINANCE_WEIGHT_B * srgbChannelToLinear(rgb.b)
    )
}

export const contrastRatio = (hexA: string, hexB: string): number | null => {
    const luminanceA = relativeLuminance(hexA)
    const luminanceB = relativeLuminance(hexB)
    if (luminanceA === null || luminanceB === null) return null
    const lighter = Math.max(luminanceA, luminanceB)
    const darker = Math.min(luminanceA, luminanceB)
    return (lighter + CONTRAST_RATIO_OFFSET) / (darker + CONTRAST_RATIO_OFFSET)
}

const foregroundContrastRatio = (foregroundHex: string, backgroundHex: string) =>
    contrastRatio(compositeOverBackground(foregroundHex, backgroundHex), backgroundHex)

type ContrastPair = {
    label: string
    foregroundKey: string
    backgroundKey: string
    /**
     * Whether a post-repair failure on this pair is fatal to a VSIX import (`validateOutputColors`)
     * or advisory-only (`validateSelectionRowContrast`, {@link repairContrastPairs}'s own attempt
     * still runs either way). Both categories apply to *any* VSIX import, audited or not — nothing
     * about `validateOutputColors`'s call sites (`convert.ts` → `vsix-theme-import.ts`,
     * `scripts/convert-vscode-theme.ts`) restricts it to the 36 bundled themes. The original 5 pairs
     * (`app`/`editor`/`panel`/`tooltip`/`matchHighlight`) are `true` — carried over unchanged from
     * before the selection-row axes below existed, per
     * `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a's "기존 5쌍 판정·수리
     * 불변" requirement: they already could reject an unrepairable import, and that behavior is
     * preserved as-is, not re-justified here. The two selection-row pairs are `false` by design:
     * promoting either to blocking would open a *new* rejection case that did not exist before this
     * batch — no fixed candidate chain can be proven to always find a contrast-clearing value (an
     * adversarial or simply unlucky upstream palette can defeat any finite chain), and doing so would
     * reopen exactly the "임포트 거부 신설" the contract's carried-over d-33 decision forbids. Repair
     * still runs unconditionally for every pair regardless of this flag; only the blocking/advisory
     * split of the *failure* differs.
     */
    blocking: boolean
}

const CONTRAST_PAIRS: readonly ContrastPair[] = [
    { label: 'app', foregroundKey: 'app.foreground', backgroundKey: 'app.background', blocking: true },
    { label: 'editor', foregroundKey: 'editor.foreground', backgroundKey: 'editor.background', blocking: true },
    { label: 'panel', foregroundKey: 'panel.sectionHeader', backgroundKey: 'panel.background', blocking: true },
    { label: 'tooltip', foregroundKey: 'app.foreground', backgroundKey: 'tooltip.background', blocking: true },
    { label: 'matchHighlight', foregroundKey: 'panel.matchHighlight', backgroundKey: 'panel.background', blocking: true },
    /**
     * Selection-row axes (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §0/§1-a).
     * `selectionMatchHighlight` shares `panel.matchHighlight` with the blocking `matchHighlight` pair
     * above — see {@link repairPair}'s `protectedBackgroundKeys` for how repair avoids clobbering that
     * already-passing pair when both fail together.
     */
    { label: 'selectionMatchHighlight', foregroundKey: 'panel.matchHighlight', backgroundKey: 'list.activeBackground', blocking: false },
    { label: 'selectionForeground', foregroundKey: 'list.foreground', backgroundKey: 'list.activeBackground', blocking: false },
]

const CONTRAST_REPAIR_BACKGROUND_CANDIDATES: Record<string, string[]> = {
    'tooltip.background': ['editorWidget.background', 'menu.background', 'dropdown.background'],
}

const CONTRAST_REPAIR_FOREGROUND_CANDIDATES: Record<string, string[]> = {
    'app.foreground': ['editor.foreground'],
    'panel.sectionHeader': ['editor.foreground'],
    'panel.matchHighlight': [
        'textLink.foreground',
        'button.background',
        'focusBorder',
        'activityBarBadge.background',
        'badge.background',
        'tab.activeBorderTop',
        'editor.foreground',
        'foreground',
    ],
    /**
     * `list.foreground`'s own mapping chain (`mapping-tables.ts`) already tried `sideBar.foreground`
     * then `foreground` — reusing either here would just re-offer the same value that already failed.
     * `editor.foreground` is the established "different, typically-opaque body-text" repair source for
     * this general (non-matchHighlight) axis, the same source `app.foreground`/`panel.sectionHeader`
     * above already use for the identical reason.
     */
    'list.foreground': ['editor.foreground'],
}

const MATCH_HIGHLIGHT_FOREGROUND_KEY = 'panel.matchHighlight'
const MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE = ', 본문 전경과 동일색 — 구별 가능한 후보 없음'

/**
 * Background keys an advisory pair's foreground repair must also stay legible against, beyond the
 * pair's own background and whatever it shares with a blocking pair (see `repairContrastPairs`'s
 * `protectedBackgroundKeys` derivation below). `list.foreground` (the `selectionForeground` pair's
 * foreground key) has no blocking-pair sibling to derive protection from — no pair in
 * `CONTRAST_PAIRS` checks `list.foreground` against anything blocking — yet the token is not
 * selection-only: `global.css`'s `--accent-foreground: var(--taide-list-foreground)` paints it over
 * `list.hoverBackground` (`--accent`) for every `hover:text-accent-foreground`/`focus:text-accent-foreground`
 * consumer (dropdown/context menus, ghost buttons), and it is also the row color for plain
 * (non-hovered, non-selected) `list.background` rows. A repair that only checks
 * `list.activeBackground` can silently reintroduce those two collapses — exactly what happened to
 * `nord`'s `list.foreground` before this fix (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md`
 * review findings d40-listfg-multisurface-regression/d40-l2-01/D40-L3-01).
 */
const ADVISORY_PROTECTED_BACKGROUND_KEYS: Record<string, string[]> = {
    selectionForeground: ['list.background', 'list.hoverBackground'],
}

const meetsMinContrast = (foregroundHex: string, backgroundHex: string) =>
    (foregroundContrastRatio(foregroundHex, backgroundHex) ?? 0) >= MIN_CONTRAST_RATIO

/**
 * Repairs a single pair, trying a background substitute first and a foreground substitute second —
 * unchanged from the pre-d40 algorithm. `protectedBackgroundKeys` is new: any background key listed
 * here must *also* stay above {@link MIN_CONTRAST_RATIO} against a chosen foreground candidate before
 * it's accepted. This exists for `panel.matchHighlight`, which two pairs now share (`matchHighlight`
 * against `panel.background`, blocking; `selectionMatchHighlight` against `list.activeBackground`,
 * advisory), and for `list.foreground` (see {@link ADVISORY_PROTECTED_BACKGROUND_KEYS}) — without
 * this guard, repairing the advisory pair could silently pick a value that fails a surface the
 * blocking pair or the row's other render paths already depended on, since a plain per-pair loop
 * only re-checks the pair it's currently on. Passing `[]` makes this identical to the original
 * single-background search.
 *
 * The no-distinctness fallback below (accepting a candidate that merely clears contrast, without
 * requiring it to read as a different color from `app.foreground`) is restricted to `pair.blocking`
 * pairs. An advisory `matchHighlight`-keyed repair (`selectionMatchHighlight`) that finds no distinct
 * candidate is left unrepaired instead — silently overwriting `panel.matchHighlight` with a
 * same-color-as-body-text value would reintroduce the exact defect
 * `docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md` §"임무 C" fixed, and could do so
 * even while the blocking `matchHighlight` pair the same token already satisfies (contrast *and*
 * distinctness) stays untouched — see review findings d40-advisory-repair-clobbers-d33-distinctness/
 * d40-l2-02. Advisory failures are allowed by design (they never block an import), so leaving the
 * pair unrepaired here is the correct outcome, not a regression.
 */
const repairPair = (
    pair: ContrastPair,
    colors: Record<string, string>,
    vscodeColors: Record<string, string>,
    protectedBackgroundKeys: readonly string[],
): { colors: Record<string, string>; repair: string | null } => {
    const foreground = colors[pair.foregroundKey]
    const background = colors[pair.backgroundKey]
    const ratio = foregroundContrastRatio(foreground, background)
    if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) return { colors, repair: null }

    const backgroundCandidates = CONTRAST_REPAIR_BACKGROUND_CANDIDATES[pair.backgroundKey] ?? []
    const repairedBackground = backgroundCandidates.map((key) => vscodeColors[key]).find((value) => value && meetsMinContrast(foreground, value))
    if (repairedBackground) {
        return {
            colors: { ...colors, [pair.backgroundKey]: repairedBackground },
            repair: `${pair.backgroundKey}: ${background} -> ${repairedBackground} (${pair.label} 대비 확보)`,
        }
    }

    const protectedBackgrounds = protectedBackgroundKeys.map((key) => colors[key])
    const satisfiesAllBackgrounds = (value: string) =>
        meetsMinContrast(value, background) && protectedBackgrounds.every((protectedBackground) => meetsMinContrast(value, protectedBackground))

    const foregroundCandidateValues = (CONTRAST_REPAIR_FOREGROUND_CANDIDATES[pair.foregroundKey] ?? []).map((key) => vscodeColors[key])
    const isMatchHighlight = pair.foregroundKey === MATCH_HIGHLIGHT_FOREGROUND_KEY
    const distinctForeground = isMatchHighlight
        ? foregroundCandidateValues.find(
              (value) =>
                  isOpaqueForegroundCandidate(value) &&
                  satisfiesAllBackgrounds(value) &&
                  isDistinctFromBodyForeground(value, colors['app.foreground']),
          )
        : undefined
    const allowNonDistinctFallback = !isMatchHighlight || pair.blocking
    const repairedForeground =
        distinctForeground ??
        (allowNonDistinctFallback ? foregroundCandidateValues.find((value) => value && satisfiesAllBackgrounds(value)) : undefined)
    if (!repairedForeground) return { colors, repair: null }

    const usedDistinctnessFallback = isMatchHighlight && !distinctForeground
    return {
        colors: { ...colors, [pair.foregroundKey]: repairedForeground },
        repair: `${pair.foregroundKey}: ${foreground} -> ${repairedForeground} (${pair.label} 대비 확보${usedDistinctnessFallback ? MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE : ''})`,
    }
}

export const repairContrastPairs = (colors: Record<string, string>, vscodeColors: Record<string, string>) => {
    const repairs: string[] = []
    let repairedColors = colors

    const blockingPairs = CONTRAST_PAIRS.filter((pair) => pair.blocking)
    for (const pair of blockingPairs) {
        const result = repairPair(pair, repairedColors, vscodeColors, [])
        repairedColors = result.colors
        if (result.repair) repairs.push(result.repair)
    }

    const advisoryPairs = CONTRAST_PAIRS.filter((pair) => !pair.blocking)
    for (const pair of advisoryPairs) {
        const sharedBlockingBackgroundKeys = blockingPairs
            .filter((blockingPair) => blockingPair.foregroundKey === pair.foregroundKey)
            .map((blockingPair) => blockingPair.backgroundKey)
        const protectedBackgroundKeys = [...sharedBlockingBackgroundKeys, ...(ADVISORY_PROTECTED_BACKGROUND_KEYS[pair.label] ?? [])]
        const result = repairPair(pair, repairedColors, vscodeColors, protectedBackgroundKeys)
        repairedColors = result.colors
        if (result.repair) repairs.push(result.repair)
    }

    return { colors: repairedColors, repairs }
}

const describeContrastViolations = (colors: Record<string, string>, pairs: readonly ContrastPair[]) =>
    pairs.flatMap((pair) => {
        const foreground = colors[pair.foregroundKey]
        const background = colors[pair.backgroundKey]
        const ratio = foregroundContrastRatio(foreground, background)
        if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) return []
        return [
            `${pair.label} 대비 부족: ${pair.foregroundKey}(${foreground}) vs ${pair.backgroundKey}(${background}) = ${ratio?.toFixed(2) ?? 'N/A'} (최소 ${MIN_CONTRAST_RATIO})`,
        ]
    })

/**
 * The import-blocking check — unchanged in behavior from before d-40. Only evaluates the 5
 * `blocking` pairs, so its result (fed straight into `convertVscodeTheme`'s `outputColorErrors`,
 * which `vsix-theme-import.ts` rejects an import on) cannot regress from the two new selection-row
 * pairs no matter what a future theme's palette looks like — see the `blocking` field's doc on
 * {@link ContrastPair} for why that's a hard requirement, not just today's empirical outcome.
 */
export const validateOutputColors = (colors: Record<string, string>) => {
    const errors: string[] = []

    if (colors['app.foreground'] === colors['app.background']) {
        errors.push(`app.foreground와 app.background가 동일한 색(${colors['app.foreground']})입니다`)
    }

    errors.push(
        ...describeContrastViolations(
            colors,
            CONTRAST_PAIRS.filter((pair) => pair.blocking),
        ),
    )

    return errors
}

/**
 * Advisory-only counterpart to {@link validateOutputColors} for the selection-row axes
 * (`panel.matchHighlight`/`list.foreground` against `list.activeBackground`). Never feeds a VSIX
 * import decision — {@link repairContrastPairs} already best-effort-repairs these pairs regardless
 * of whether this function is even called; this exists so the bundled-theme catalog gate
 * (`bundled-theme-contrast.test.ts`) and the Rust catalog lint (d-40 §1-c) can still audit them.
 */
export const validateSelectionRowContrast = (colors: Record<string, string>) =>
    describeContrastViolations(
        colors,
        CONTRAST_PAIRS.filter((pair) => !pair.blocking),
    )
