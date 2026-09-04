import type { ProjectRef } from '@shared/api/bindings'
import {
    PROJECT_COLOR_TOKENS,
    PROJECT_COLOR_VAR_PREFIX,
    PROJECT_LABEL_CLASS_BY_LENGTH,
    PROJECT_LABEL_MAX_CODEPOINTS,
} from '@shared/constants/project-display'
import type { ProjectColorToken } from '@shared/constants/project-display'

/**
 * Which of the three mutually exclusive presentations a project's sidebar button draws. `default` is
 * the plain folder glyph — the appearance every project had before display overrides existed.
 */
export type ProjectDisplayMode = 'label' | 'icon' | 'default'

export type ProjectDisplayResolution = {
    mode: ProjectDisplayMode
    label: string | null
    icon: string | null
    colorVar: string | null
}

/**
 * Anything carrying the persisted overrides — `ProjectRef` (what the sidebar lists) and `Project`
 * (the record `project_get` returns) both satisfy it, and both spell the field
 * `display?: ProjectDisplay` because specta emits `#[serde(default)]` fields as optional.
 */
type ProjectDisplaySource = Pick<ProjectRef, 'display'>

const CONTROL_CHARACTER_PATTERN = /\p{Cc}/gu

const stripControlCharacters = (value: string) => value.replace(CONTROL_CHARACTER_PATTERN, '')

const toCodepointLimitedText = (value: string) => [...value].slice(0, PROJECT_LABEL_MAX_CODEPOINTS).join('')

/**
 * What the label input may hold while typing: control characters dropped and the length capped in
 * codepoints (`maxLength` counts UTF-16 units, so it alone would let a 4-codepoint emoji label be
 * cut mid-surrogate). Deliberately does *not* trim — trimming on every keystroke would eat the space
 * a user types between two characters of a label like `A B`.
 */
export const clampProjectLabel = (value: string) => toCodepointLimitedText(stripControlCharacters(value))

/**
 * What is actually persisted: {@link clampProjectLabel}'s cleaning plus the surrounding-whitespace
 * trim, in the same order `domain::project::service::sanitize_display_label` applies it (strip, trim,
 * then measure), so the dialog can never submit a value the backend would reject.
 */
export const normalizeProjectLabel = (value: string) => toCodepointLimitedText(stripControlCharacters(value).trim())

const asNonEmptyText = (value: string | null | undefined) => {
    const trimmed = value?.trim() ?? ''
    return trimmed.length > 0 ? trimmed : null
}

const isProjectColorToken = (value: string): value is ProjectColorToken => PROJECT_COLOR_TOKENS.some((token) => token === value)

/**
 * The stored color as a token the picker can preselect, or `null` when nothing valid is stored —
 * the one place a persisted color string is validated against {@link PROJECT_COLOR_TOKENS}.
 */
export const resolveProjectColorToken = (value: string | null | undefined) => {
    const token = asNonEmptyText(value)
    return token !== null && isProjectColorToken(token) ? token : null
}

/**
 * The single place a persisted `ProjectDisplay` becomes something renderable — every consumer reads
 * this result instead of the optional binding fields. specta emits `display?:` (like `capabilities?:`
 * and `rootMissing?:`), and the audit's R5#5 finding was that per-call-site `??` defaults drift apart
 * (`agentStatusBadgeEnabled` ended up `?? true` in one file and `?? false` in another); funnelling
 * every fallback through one function is what makes that impossible here.
 *
 * The axes are ranked, not combined: a label wins over an icon because both occupy the same 40px
 * button, matching the exclusive icon/label/default choice the display dialog offers. Color is
 * orthogonal and survives all three modes — it tints the glyph or the text, never the background, so
 * it cannot collapse the contrast of the active-project highlight.
 *
 * An icon id is passed through verbatim rather than validated here: whether the catalog still has it
 * is `project-icon-registry`'s question, and it answers with the `folder` fallback.
 */
export const resolveProjectDisplay = ({ display }: ProjectDisplaySource): ProjectDisplayResolution => {
    const label = asNonEmptyText(display?.label)
    const icon = asNonEmptyText(display?.icon)
    const colorToken = resolveProjectColorToken(display?.color)
    const colorVar = colorToken === null ? null : `var(${PROJECT_COLOR_VAR_PREFIX}${colorToken})`

    if (label !== null) return { mode: 'label', label: toCodepointLimitedText(label), icon: null, colorVar }
    if (icon !== null) return { mode: 'icon', label: null, icon, colorVar }
    return { mode: 'default', label: null, icon: null, colorVar }
}

/**
 * Whether the project has any override at all — what gates the context menu's "reset to default".
 * Derived from the resolution rather than the raw record so a stored value the sidebar cannot render
 * (an unknown color token) does not advertise a reset that would visibly change nothing.
 */
export const isProjectDisplayCustomized = (resolution: ProjectDisplayResolution) => resolution.mode !== 'default' || resolution.colorVar !== null

export const resolveProjectLabelClassName = (label: string) =>
    PROJECT_LABEL_CLASS_BY_LENGTH[Math.min([...label].length, PROJECT_LABEL_CLASS_BY_LENGTH.length - 1)]
