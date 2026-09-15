import { compositeOverBackground, deltaE76, hexToRgb, isHexColor, rgbToHex } from '@shared/lib/color'
import type { ComponentContrastPair } from '@shared/lib/theme-convert/component-contrast-pairs'
import {
    COMPONENT_CONTRAST_EXEMPTIONS,
    COMPONENT_CONTRAST_PAIRS,
    FIXED_FOREGROUND_CONTRAST_PAIRS,
} from '@shared/lib/theme-convert/component-contrast-pairs'
import { isDistinctFromBodyForeground, isOpaqueForegroundCandidate } from '@shared/lib/theme-convert/mapping-tables'
import { STATE_MIN_DISTINCT_DELTA_E } from '@shared/lib/theme-convert/state-distinctness-pairs'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'

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

export type ContrastPair = {
    label: string
    foregroundKey: string
    backgroundKey: string
    /**
     * Whether a post-repair failure on this pair is fatal to a VSIX import (`validateOutputColors`)
     * or advisory-only (`validateSelectionRowContrast`, {@link repairContrastPairs}'s own attempt
     * still runs either way). Both categories apply to *any* VSIX import, audited or not — nothing
     * about `validateOutputColors`'s call sites (`convert.ts` → `vsix-theme-import.ts`,
     * `scripts/convert-vscode-theme.ts`) restricts it to the 47 bundled themes. The original 5 pairs
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

export const CONTRAST_PAIRS: readonly ContrastPair[] = [
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

/**
 * Runs the blocking pairs, then the advisory selection-row pairs, then
 * {@link repairComponentContrast}. The component pass goes last because it reads the backgrounds the
 * earlier passes may have moved (`tooltip.background` is the only one they write) and because its
 * own writes are foreground tokens none of the earlier pairs measure against — so neither pass can
 * undo the other regardless of what a theme's palette looks like.
 */
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

    const componentResult = repairComponentContrast(repairedColors)

    return { colors: componentResult.colors, repairs: [...repairs, ...componentResult.repairs] }
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

/**
 * The root surface every other surface is ultimately painted on. No bundled theme gives
 * `app.background` an alpha channel (the converter maps it straight from `editor.background`), which
 * is what lets {@link resolveComponentSurface} resolve a translucent surface against something
 * concrete instead of recursing.
 */
const APP_BACKGROUND_KEY = 'app.background'

/**
 * The key of the terminal background inside the flattened xterm record — the `background` entry
 * `TERMINAL_MIRRORED_TOKENS` (`ansi-palette.ts`) writes from `colors['terminal.background']`.
 */
const TERMINAL_BACKGROUND_TOKEN = 'background'

/**
 * Number of 8-bit steps the foreground→pole segment is sampled at in
 * {@link deriveContrastingForeground}. sRGB hex output is quantized to 256 levels per channel, so a
 * finer grid could not produce a different color.
 */
const CONTRAST_MIX_STEPS = 255

/**
 * The two ends a failing foreground is moved toward. Mixing with black scales all three channels by
 * the same factor and mixing with white moves them toward the same ceiling, so both keep the hue the
 * theme chose while changing only how light the color reads — which is the only thing WCAG contrast
 * measures. Both are searched and the smaller perceptual change wins, so a near-white foreground on
 * a mid-tone background is nudged to white rather than flipped to black.
 */
const CONTRAST_REPAIR_POLES = ['#000000', '#ffffff']

/**
 * Component pairs whose repair moves the *background* instead of the text, keyed by background token
 * and listing the tokens to substitute in order. The one entry is `menu.itemHover`: its pair's
 * foreground is `app.foreground`, the global body color every surface in the app is measured
 * against, so the rule the rest of this table follows ("move the text") would repaint the whole UI
 * to fix a menu row. `list.hoverBackground` is the substitute because it is the theme's own
 * row-hover tint, designed to sit under unchanged body text, and because it is what
 * `mapping-tables.ts` now resolves `menu.itemHover` from in the first place — so a re-converted
 * theme and a repaired one land on the same value.
 */
const COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES: Record<string, string[]> = {
    'menu.itemHover': ['list.hoverBackground'],
}

const resolveComponentSurface = (colors: Record<string, string>, surfaceKey: string) => {
    const surface = colors[surfaceKey]
    if (!surface || !isHexColor(surface)) return null
    if (surfaceKey === APP_BACKGROUND_KEY) return surface
    const root = colors[APP_BACKGROUND_KEY]
    if (!root || !isHexColor(root)) return surface
    return compositeOverBackground(surface, root)
}

/**
 * The opaque color a pair's background actually renders as — see
 * {@link ComponentContrastPair.surfaceKey}. Returns `null` when the background or its surface is
 * missing or is not a hex color (`transparent`, or an unresolved `@palette` reference), which makes
 * the pair unmeasurable rather than violating.
 */
const resolveComponentBackground = (pair: ComponentContrastPair, colors: Record<string, string>) => {
    const background = colors[pair.backgroundKey]
    if (!background || !isHexColor(background)) return null
    const surface = resolveComponentSurface(colors, pair.surfaceKey)
    return surface ? compositeOverBackground(background, surface) : background
}

const measureComponentPair = (pair: ComponentContrastPair, colors: Record<string, string>) => {
    const foreground = colors[pair.foregroundKey]
    if (!foreground || !isHexColor(foreground)) return null
    const background = resolveComponentBackground(pair, colors)
    if (background === null) return null
    return { background, ratio: foregroundContrastRatio(foreground, background) }
}

/**
 * Reports every (text color, surface) pair of {@link COMPONENT_CONTRAST_PAIRS} whose text cannot be
 * read where the UI actually draws it. Advisory by design: unlike {@link validateOutputColors} this
 * never feeds an import decision — {@link repairContrastPairs} repairs these pairs during conversion,
 * and the catalog gate (`bundled-theme-contrast.test.ts`) plus the Rust catalog lint are what hold
 * the bundled data to it. The message prints the composited background next to the declared one when
 * they differ, so a failure caused by a translucent surface is readable from the message alone.
 */
export const validateComponentContrast = (colors: Record<string, string>) =>
    COMPONENT_CONTRAST_PAIRS.flatMap((pair) => {
        const measurement = measureComponentPair(pair, colors)
        if (!measurement || (measurement.ratio !== null && measurement.ratio >= MIN_CONTRAST_RATIO)) return []
        const declared = colors[pair.backgroundKey]
        const rendered = measurement.background === declared ? '' : ` -> ${measurement.background}`
        return [
            `${pair.label} 대비 부족: ${pair.foregroundKey}(${colors[pair.foregroundKey]}) vs ${pair.backgroundKey}(${declared}${rendered}) = ${measurement.ratio?.toFixed(2) ?? 'N/A'} (최소 ${MIN_CONTRAST_RATIO})`,
        ]
    })

const mixTowardPole = (fromHex: string, poleHex: string, ratio: number) => {
    const from = hexToRgb(fromHex)
    const pole = hexToRgb(poleHex)
    if (!from || !pole) return null
    return rgbToHex({
        r: from.r + (pole.r - from.r) * ratio,
        g: from.g + (pole.g - from.g) * ratio,
        b: from.b + (pole.b - from.b) * ratio,
    })
}

/**
 * Derives a replacement text color that clears {@link MIN_CONTRAST_RATIO} against *every* surface
 * the token is drawn on, by walking away from the current color toward each of
 * {@link CONTRAST_REPAIR_POLES} and taking the first step that satisfies all of them. Unlike the
 * candidate substitution the blocking pairs use, this keeps the theme's own hue: a washed-out green
 * git decoration comes back as a deeper green rather than as the body text color. Scanning step by
 * step (rather than bisecting) is what makes the result the *smallest* change on each ray —
 * contrast against a surface is not monotonic along a ray when that surface's luminance sits between
 * the two ends, so a bisection could skip past the nearest solution. Returns `null` when neither
 * pole satisfies every surface, which happens when one token is asked to be legible on two surfaces
 * that are themselves at opposite ends of the luminance range; the caller leaves the pair
 * unrepaired rather than fixing one surface by breaking another.
 */
const deriveContrastingForeground = (foregroundHex: string, backgroundHexes: readonly string[]) => {
    const base = compositeOverBackground(foregroundHex, backgroundHexes[0])
    const satisfiesAll = (candidate: string) => backgroundHexes.every((background) => meetsMinContrast(candidate, background))

    let best: { value: string; distance: number } | null = null
    for (const pole of CONTRAST_REPAIR_POLES) {
        for (let step = 1; step <= CONTRAST_MIX_STEPS; step += 1) {
            const candidate = mixTowardPole(base, pole, step / CONTRAST_MIX_STEPS)
            if (!candidate || !satisfiesAll(candidate)) continue
            const distance = deltaE76(candidate, base) ?? Number.POSITIVE_INFINITY
            if (!best || distance < best.distance) best = { value: candidate, distance }
            break
        }
    }
    return best?.value ?? null
}

const toMeasurableSurfaces = (surfaces: readonly (string | null | undefined)[]) =>
    surfaces.filter((surface): surface is string => surface !== null && surface !== undefined && isHexColor(surface))

/** Whether a pair is repaired by substituting its background rather than by moving its text color. */
const isBackgroundRepairedPair = (pair: ComponentContrastPair) => pair.backgroundKey in COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES

/**
 * The surfaces a foreground token is measured against here — the component pairs that name it,
 * minus the ones repaired from the background side. Leaving those in would let a menu row's
 * shortfall drag the global body color across the whole app, which is the reason they are
 * background-repaired to begin with.
 */
const collectComponentSurfaces = (foregroundKey: string, colors: Record<string, string>) =>
    toMeasurableSurfaces(
        COMPONENT_CONTRAST_PAIRS.filter((pair) => pair.foregroundKey === foregroundKey && !isBackgroundRepairedPair(pair)).map((pair) =>
            resolveComponentBackground(pair, colors),
        ),
    )

/**
 * Substitutes the background of every pair in {@link COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES} that
 * cannot carry its text, taking the first candidate that both clears {@link MIN_CONTRAST_RATIO} and
 * stays {@link STATE_MIN_DISTINCT_DELTA_E} away from the pair's surface — a hover tint that reads as
 * the surface it sits on would trade this lint's failure for `state-distinctness.ts`'s. A candidate
 * that does not clear is not applied at all, so a substitution never makes a pair worse than the
 * value the theme shipped; the pair is then left failing for the catalog gate to account for.
 */
const repairComponentBackgrounds = (colors: Record<string, string>) => {
    const repairs: string[] = []
    let repairedColors = colors

    for (const pair of COMPONENT_CONTRAST_PAIRS.filter(isBackgroundRepairedPair)) {
        const measurement = measureComponentPair(pair, repairedColors)
        if (!measurement || (measurement.ratio !== null && measurement.ratio >= MIN_CONTRAST_RATIO)) continue

        const surface = resolveComponentSurface(repairedColors, pair.surfaceKey)
        const foreground = repairedColors[pair.foregroundKey]
        const replacement = COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES[pair.backgroundKey]
            .map((key) => repairedColors[key])
            .find((candidate) => {
                if (!candidate || !isHexColor(candidate)) return false
                const rendered = surface ? compositeOverBackground(candidate, surface) : candidate
                if (!meetsMinContrast(foreground, rendered)) return false
                return surface === null || (deltaE76(rendered, surface) ?? 0) >= STATE_MIN_DISTINCT_DELTA_E
            })
        if (!replacement) continue

        const replaced = repairedColors[pair.backgroundKey]
        repairedColors = { ...repairedColors, [pair.backgroundKey]: replacement }
        repairs.push(`${pair.backgroundKey}: ${replaced} -> ${replacement} (${pair.label} 대비 확보)`)
    }

    return { colors: repairedColors, repairs }
}

/**
 * The surfaces of {@link CONTRAST_PAIRS} that share this foreground token — `list.foreground` is both
 * `selectionForeground`'s foreground and two component rows'. They constrain a replacement without
 * ever triggering one: a repair here must not hand back a value that breaks the selection row the
 * d-40 batch fixed, but neither may it repair that row, because
 * `bundled-theme-contrast.test.ts`'s exemption registry pins exactly which themes still fail it and
 * why (`everforest-light`/`rose-pine-dawn`, upstream palettes with a single shade per accent). Read
 * raw, exactly as {@link repairPair} reads them, so this adds a constraint to that pair rather than
 * re-interpreting it.
 */
const collectLegacySurfaces = (foregroundKey: string, colors: Record<string, string>) =>
    toMeasurableSurfaces(CONTRAST_PAIRS.filter((pair) => pair.foregroundKey === foregroundKey).map((pair) => colors[pair.backgroundKey]))

/**
 * Repairs the component pairs by foreground token rather than pair by pair, so a token drawn on
 * several surfaces (`statusIndicator.error` on both the status bar strip and the problems panel,
 * `appSidebar.iconDefault` on three) is moved once to a value that satisfies all of them at once
 * instead of being pulled back and forth. {@link repairComponentBackgrounds} runs first for the
 * handful of pairs repaired from the background side, since moving a background changes what the
 * foregrounds after it are measured against, while the foreground pass writes no background.
 * One pass is enough and the result needs no fixed-point
 * loop: no token in {@link COMPONENT_CONTRAST_PAIRS} is both a foreground and a background, so a
 * repair never changes the input of a later one — `bundled-theme-contrast.test.ts` locks that
 * property in. Deterministic and idempotent: re-running over repaired colors reports no repairs and
 * returns the same object, which is what lets `scripts/repair-theme-contrast.ts` be re-run at any
 * time. A repaired token is always opaque, since the value that was measured is the one the surface
 * composites to — keeping an alpha channel would re-introduce a color that reads differently on each
 * of the surfaces the single repaired value now has to serve.
 */
export const repairComponentContrast = (colors: Record<string, string>) => {
    const backgroundResult = repairComponentBackgrounds(colors)
    const repairs = [...backgroundResult.repairs]
    let repairedColors = backgroundResult.colors

    const foregroundKeys = [...new Set(COMPONENT_CONTRAST_PAIRS.map((pair) => pair.foregroundKey))]
    for (const foregroundKey of foregroundKeys) {
        const foreground = repairedColors[foregroundKey]
        if (!foreground || !isHexColor(foreground)) continue
        const surfaces = collectComponentSurfaces(foregroundKey, repairedColors)
        if (surfaces.length === 0 || surfaces.every((surface) => meetsMinContrast(foreground, surface))) continue

        const replacement = deriveContrastingForeground(foreground, [...surfaces, ...collectLegacySurfaces(foregroundKey, repairedColors)])
        if (!replacement || replacement === foreground) continue

        repairedColors = { ...repairedColors, [foregroundKey]: replacement }
        const labels = COMPONENT_CONTRAST_PAIRS.filter((pair) => pair.foregroundKey === foregroundKey)
            .map((pair) => pair.label)
            .join('/')
        repairs.push(`${foregroundKey}: ${foreground} -> ${replacement} (${labels} 대비 확보)`)
    }

    return { colors: repairedColors, repairs }
}

/**
 * ANSI palette legibility against the terminal's own background, reported for the record and never
 * gated or repaired (`docs/theme-system.md` §8.6). Two reasons, both measured over the bundled
 * catalog: the achromatic ends collapse into the background by terminal convention rather than by
 * mistake (`black` falls below the threshold in 34 of the 47 bundled themes, `brightBlack` in 21,
 * `brightWhite` in 12 — a shell that prints black on a black background is behaving the way every
 * other terminal does), and the chromatic colors are the theme's palette identity, reused by
 * `graph.lane*`, the agent status icons and the syntax fallbacks, so moving them to clear a
 * threshold would repaint far more than the terminal. `terminal` is the flattened xterm record
 * (`resolve-terminal.ts`), not `colors`.
 */
/**
 * Whether a {@link validateComponentContrast} message is one of the bundled catalog's registered
 * shortfalls ({@link COMPONENT_CONTRAST_EXEMPTIONS}). Matches on the pair label the message opens
 * with, so a registry entry for one axis never silences a different one on the same theme. Only the
 * bundled-catalog callers (the gate test, the repair script) use it — an imported theme has no id in
 * the registry and is audited unfiltered.
 */
export const isExemptComponentContrastViolation = (themeId: string, violation: string) =>
    Object.keys(COMPONENT_CONTRAST_EXEMPTIONS).some((key) => {
        const [exemptId, label] = key.split(':')
        return exemptId === themeId && violation.startsWith(`${label} `)
    })

/**
 * Legibility of the component colors a theme cannot influence
 * ({@link FIXED_FOREGROUND_CONTRAST_PAIRS}), reported for the record and never gated or repaired —
 * the foreground is a literal in a component, so there is nothing in the theme to move. The bundled
 * catalog's measurements are recorded in `docs/theme-system.md` §8.6.
 */
export const validateFixedForegroundContrast = (colors: Record<string, string>) =>
    FIXED_FOREGROUND_CONTRAST_PAIRS.flatMap((pair) => {
        const background = colors[pair.backgroundKey]
        if (!background || !isHexColor(background)) return []
        const surface = resolveComponentSurface(colors, pair.surfaceKey)
        const rendered = surface ? compositeOverBackground(background, surface) : background
        const ratio = foregroundContrastRatio(pair.foregroundValue, rendered)
        if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) return []
        return [
            `${pair.label} 대비 낮음: ${pair.foregroundValue} vs ${pair.backgroundKey}(${background}) = ${ratio?.toFixed(2) ?? 'N/A'} (기준 ${MIN_CONTRAST_RATIO})`,
        ]
    })

export const validateTerminalAnsiContrast = (terminal: Record<string, string>) => {
    const background = terminal[TERMINAL_BACKGROUND_TOKEN]
    if (!background || !isHexColor(background)) return []
    return TERMINAL_ANSI_TOKENS.flatMap((token) => {
        const value = terminal[token]
        if (!value || !isHexColor(value)) return []
        const ratio = foregroundContrastRatio(value, background)
        if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) return []
        return [
            `ansi ${token} 대비 낮음: ${value} vs ${TERMINAL_BACKGROUND_TOKEN}(${background}) = ${ratio?.toFixed(2) ?? 'N/A'} (기준 ${MIN_CONTRAST_RATIO})`,
        ]
    })
}
