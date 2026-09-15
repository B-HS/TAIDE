import { ALPHA_CHANNEL_MAX, compositeOverBackground, deltaE76, HEX_ALPHA_LENGTH, hexToRgb, isHexColor, rgbToHex } from '@shared/lib/color'
import type { StateDistinctnessPair } from '@shared/lib/theme-convert/state-distinctness-pairs'
import { APP_SHADOW_MIN_ALPHA, STATE_DISTINCTNESS_PAIRS } from '@shared/lib/theme-convert/state-distinctness-pairs'

const BODY_FOREGROUND_KEY = 'app.foreground'

const SHADOW_KEY = 'app.shadow'

/** Radix of the two hex digits `app.shadow`'s alpha channel is written in. */
const HEX_RADIX = 16

/**
 * The color a shadow below {@link APP_SHADOW_MIN_ALPHA} is replaced by: black at exactly that alpha.
 * Black rather than the declared RGB because a token whose alpha is zeroed carries no usable color —
 * `tokyo-night` ships `#ffffff00`, and raising *that* alpha would paint a white glow and a white
 * modal scrim over a dark theme. Every shadow default this converter and VS Code ship is black
 * (`mapping-tables.ts`'s `SAFE_DEFAULT_COLORS`), and a shadow that darkens whatever is behind it is
 * correct on both theme types.
 */
const SHADOW_REPAIR_RGB = '#000000'

/**
 * Number of 8-bit steps the container→foreground segment is sampled at when deriving a replacement
 * state color. sRGB hex output is quantized to 256 levels per channel anyway, so a finer grid could
 * not produce a different color.
 */
const MIX_RATIO_STEPS = 255

/**
 * How many times {@link repairStateDistinctness} re-runs the whole table. One pass is not enough
 * because several pairs share a token (`input.border` is checked against three containers,
 * `button.primaryBackground` against three) and a repair made for one pair changes the input of
 * every later pair that reads the same token. Passes stop as soon as a full sweep makes no change,
 * so the usual cost is one sweep plus one confirming sweep; the cap only bounds the pathological
 * case where two pairs would push the same token back and forth forever.
 */
const REPAIR_MAX_PASSES = 4

/**
 * The opaque surface a pair's tokens are painted on, or `null` when the pair declares none or the
 * declared one is not a hex color.
 */
const resolveSurface = (pair: StateDistinctnessPair, colors: Record<string, string>) => {
    const surface = pair.surfaceKey ? colors[pair.surfaceKey] : undefined
    return surface && isHexColor(surface) ? surface : null
}

/**
 * The two opaque colors a pair is compared at: the container as it renders, and the base the state
 * color is composited over before the comparison.
 *
 * For a normal pair the state is painted *on* the container, so its base is the container itself —
 * a container that carries alpha is resolved against `surfaceKey` first (see
 * {@link StateDistinctnessPair.surfaceKey}). For a {@link StateDistinctnessPair.sibling} pair the two
 * tokens are peers on the same surface, so both are composited over that surface instead.
 *
 * Returns `null` when a token is missing or is not a hex color (`transparent`, or a `@palette`
 * reference that was never resolved), which makes the pair unmeasurable rather than violating.
 */
const resolvePairSurfaces = (pair: StateDistinctnessPair, colors: Record<string, string>) => {
    const container = colors[pair.containerKey]
    if (!container || !isHexColor(container)) return null
    const surface = resolveSurface(pair, colors)
    const containerHex = surface ? compositeOverBackground(container, surface) : container
    return { containerHex, stateBaseHex: pair.sibling ? (surface ?? containerHex) : containerHex }
}

/**
 * Perceptual distance between a state color as it renders on `stateBaseHex` and the container it is
 * compared against. The state is composited over its base first, so an `#rrggbbaa` overlay is
 * measured as the color the user sees rather than as its raw RGB.
 */
const measureDistance = (stateHex: string | undefined, containerHex: string, stateBaseHex: string) => {
    if (!stateHex || !isHexColor(stateHex)) return null
    return deltaE76(compositeOverBackground(stateHex, stateBaseHex), containerHex)
}

type PairMeasurement = {
    containerHex: string
    stateBaseHex: string
    distance: number
    isDistinct: boolean
}

const measurePair = (pair: StateDistinctnessPair, colors: Record<string, string>) => {
    const surfaces = resolvePairSurfaces(pair, colors)
    if (!surfaces) return null
    const { containerHex, stateBaseHex } = surfaces
    const distance = measureDistance(colors[pair.stateKey], containerHex, stateBaseHex)
    if (distance === null) return null
    const alternativeDistance = pair.alternativeStateKey ? measureDistance(colors[pair.alternativeStateKey], containerHex, stateBaseHex) : null
    const isDistinct = distance >= pair.minDeltaE || (alternativeDistance ?? 0) >= pair.minDeltaE
    return { containerHex, stateBaseHex, distance, isDistinct }
}

const describeViolation = (pair: StateDistinctnessPair, colors: Record<string, string>, measurement: PairMeasurement) =>
    `${pair.label} 구별성 부족: ${pair.stateKey}(${colors[pair.stateKey]}) vs ${pair.containerKey}(${colors[pair.containerKey]}) = ΔE ${measurement.distance.toFixed(2)} (최소 ${pair.minDeltaE})`

/**
 * The alpha `app.shadow` renders at. A 6-digit value is opaque; anything that is not a hex color at
 * all (`transparent`, an unresolved `@palette` reference) returns `null` and is left alone, the same
 * "unmeasurable rather than violating" rule the pairs above follow.
 */
const shadowAlpha = (shadowHex: string | undefined) => {
    if (!shadowHex || !isHexColor(shadowHex)) return null
    if (shadowHex.length !== HEX_ALPHA_LENGTH) return 1
    return Number.parseInt(shadowHex.slice(HEX_ALPHA_LENGTH - 2, HEX_ALPHA_LENGTH), HEX_RADIX) / ALPHA_CHANNEL_MAX
}

/**
 * The shadow axis of this lint. `app.shadow` has no container to be compared against — it is drawn
 * over every surface in the app — so what makes it collapse is its own alpha rather than a ΔE
 * against another token, and {@link APP_SHADOW_MIN_ALPHA} is the bound.
 */
const describeShadowViolation = (colors: Record<string, string>) => {
    const alpha = shadowAlpha(colors[SHADOW_KEY])
    if (alpha === null || alpha >= APP_SHADOW_MIN_ALPHA) return []
    return [`appShadow 구별성 부족: ${SHADOW_KEY}(${colors[SHADOW_KEY]}) 알파 ${alpha.toFixed(3)} (최소 ${APP_SHADOW_MIN_ALPHA})`]
}

const repairShadow = (colors: Record<string, string>) => {
    const shadow = colors[SHADOW_KEY]
    const alpha = shadowAlpha(shadow)
    if (alpha === null || alpha >= APP_SHADOW_MIN_ALPHA) return { colors, repair: null }

    const replacement = `${SHADOW_REPAIR_RGB}${Math.round(APP_SHADOW_MIN_ALPHA * ALPHA_CHANNEL_MAX)
        .toString(HEX_RADIX)
        .padStart(2, '0')}`
    const repairedColors: Record<string, string> = { ...colors, [SHADOW_KEY]: replacement }
    return {
        colors: repairedColors,
        repair: `${SHADOW_KEY}: ${shadow} -> ${replacement} (appShadow 구별성 확보)`,
    }
}

/**
 * Reports every (state, container) pair of {@link STATE_DISTINCTNESS_PAIRS} whose state has collapsed
 * into the surface it is drawn on, plus the one axis that has no container to be compared against
 * ({@link APP_SHADOW_MIN_ALPHA}, the shadow that separates every floating surface from the content
 * behind it). Advisory by design: unlike `contrast.ts`'s `validateOutputColors`,
 * this never feeds an import decision — `convertVscodeTheme` repairs these pairs before reporting
 * them, and the catalog gate (`bundled-theme-state-distinctness.test.ts`) plus the Rust catalog lint
 * are what actually hold the bundled data to it.
 */
export const validateStateDistinctness = (colors: Record<string, string>) => [
    ...STATE_DISTINCTNESS_PAIRS.flatMap((pair) => {
        const measurement = measurePair(pair, colors)
        if (!measurement || measurement.isDistinct) return []
        return [describeViolation(pair, colors, measurement)]
    }),
    ...describeShadowViolation(colors),
]

const mixTowardForeground = (containerHex: string, foregroundHex: string, ratio: number) => {
    const container = hexToRgb(containerHex)
    const foreground = hexToRgb(foregroundHex)
    if (!container || !foreground) return null
    return rgbToHex({
        r: container.r + (foreground.r - container.r) * ratio,
        g: container.g + (foreground.g - container.g) * ratio,
        b: container.b + (foreground.b - container.b) * ratio,
    })
}

/**
 * Derives a replacement state color by walking from the container toward the theme's own body text
 * color until the pair clears `minDeltaE`, and returns the closest such color to the container — the
 * smallest visual change that restores the state. The search is a binary search over the 256 sRGB
 * steps of that segment which only ever moves its upper bound onto a step it has verified, so the
 * returned color always clears the threshold; it is the *smallest* such step as long as distance
 * grows with the mix ratio, which holds for every repair the bundled catalog needs (recorded in
 * `docs/theme-system.md` §8.5). Returns `null` when even the body foreground itself cannot clear the
 * threshold, leaving the pair unrepaired rather than substituting a color that would not fix it.
 */
const deriveDistinctState = (containerHex: string, foregroundHex: string, minDeltaE: number) => {
    const mixAt = (step: number) => mixTowardForeground(containerHex, foregroundHex, step / MIX_RATIO_STEPS)
    const satisfiesAt = (step: number) => {
        const candidate = mixAt(step)
        return candidate !== null && (measureDistance(candidate, containerHex, containerHex) ?? 0) >= minDeltaE
    }

    if (!satisfiesAt(MIX_RATIO_STEPS)) return null

    let rejected = 0
    let accepted = MIX_RATIO_STEPS
    while (accepted - rejected > 1) {
        const middle = Math.floor((rejected + accepted) / 2)
        if (satisfiesAt(middle)) accepted = middle
        else rejected = middle
    }
    return mixAt(accepted)
}

/**
 * Picks the palette entry closest to the color being replaced among those that clear `minDeltaE`
 * against the container, so a theme that names its own colors keeps using them instead of gaining a
 * derived shade. Ties are broken by palette key so the choice never depends on object iteration
 * order. Converted VS Code themes ship an empty `palette` (`scripts/convert-vscode-theme.ts` writes
 * `palette: {}`), so this path only fires for hand-authored and user-saved themes — the format's
 * `@name` references (`mapping-tables.ts`'s `SELF_REF_PREFIX`, `service.rs`'s `resolve_value`) are
 * what make a palette a real part of a theme file rather than a vestige.
 */
const pickPaletteReplacement = (
    palette: Record<string, string>,
    replacedHex: string,
    { containerHex, stateBaseHex }: Pick<PairMeasurement, 'containerHex' | 'stateBaseHex'>,
    minDeltaE: number,
) => {
    const candidates = Object.entries(palette)
        .filter(([, value]) => isHexColor(value))
        .filter(([, value]) => (measureDistance(value, containerHex, stateBaseHex) ?? 0) >= minDeltaE)
        .map(([key, value]) => ({ key, value, distance: deltaE76(value, replacedHex) ?? Number.POSITIVE_INFINITY }))
        .sort((left, right) => left.distance - right.distance || (left.key < right.key ? -1 : 1))
    return candidates[0]?.value
}

/**
 * Repairs the state side of a pair, never the container — a container is a surface other tokens are
 * measured against, so moving it would trade one collapsed pair for several. When a pair declares an
 * {@link StateDistinctnessPair.alternativeStateKey} the alternative is what gets repaired instead:
 * a theme whose active tab is the same color as the tab strip is making a deliberate flat-chrome
 * choice, and the alternative (`tabBar.tabActiveIndicator`, which `features/tab/tab-item.tsx` always
 * renders for the active tab) exists precisely to carry the distinction that design gives up.
 */
const repairPair = (pair: StateDistinctnessPair, colors: Record<string, string>, palette: Record<string, string>) => {
    const measurement = measurePair(pair, colors)
    if (!measurement || measurement.isDistinct) return { colors, repair: null }

    const repairedKey = pair.alternativeStateKey ?? pair.stateKey
    const replaced = colors[repairedKey]
    const foreground = colors[BODY_FOREGROUND_KEY]
    if (!foreground || !isHexColor(foreground)) return { colors, repair: null }

    const replacement =
        pickPaletteReplacement(palette, replaced ?? measurement.containerHex, measurement, pair.minDeltaE) ??
        deriveDistinctState(measurement.containerHex, foreground, pair.minDeltaE)
    if (!replacement || replacement === replaced) return { colors, repair: null }

    return {
        colors: { ...colors, [repairedKey]: replacement },
        repair: `${repairedKey}: ${replaced} -> ${replacement} (${pair.label} 구별성 확보)`,
    }
}

/**
 * Runs {@link repairPair} over the whole table until it stops changing anything, then lifts a
 * collapsed `app.shadow`, so the output satisfies {@link validateStateDistinctness} on every axis
 * rather than on the last pair repaired. The shadow pass runs last and once: it reads and writes a
 * token no pair uses as a state, container or surface, so it can neither be undone by the loop nor
 * undo it.
 * Deterministic and idempotent: repairing an already-repaired set of colors reports no repairs and
 * returns the same object, which is what lets `scripts/repair-theme-state-distinctness.ts` be re-run
 * over the bundled themes at any time.
 */
export const repairStateDistinctness = (colors: Record<string, string>, palette: Record<string, string>) => {
    const repairs: string[] = []
    let repairedColors = colors

    for (let pass = 0; pass < REPAIR_MAX_PASSES; pass += 1) {
        const passStart = repairedColors
        for (const pair of STATE_DISTINCTNESS_PAIRS) {
            const result = repairPair(pair, repairedColors, palette)
            repairedColors = result.colors
            if (result.repair) repairs.push(result.repair)
        }
        if (repairedColors === passStart) break
    }

    const shadowResult = repairShadow(repairedColors)
    if (shadowResult.repair) repairs.push(shadowResult.repair)

    return { colors: shadowResult.colors, repairs }
}
