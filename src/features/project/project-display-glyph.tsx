import type { FC } from 'react'
import { cn } from '@shared/lib/cn'
import type { ProjectDisplayResolution } from '@shared/lib/project-display'
import { resolveProjectLabelClassName } from '@shared/lib/project-display'
import { PROJECT_ICON_COMPONENT_MAP, resolveProjectIconName } from '@shared/icons/project-icon-registry'

type ProjectDisplayGlyphProps = {
    display: ProjectDisplayResolution
}

/**
 * The 40px-button contents alone — no button, no badge, no active indicator — so the sidebar item
 * and the display dialog's live preview draw the same three-step ladder (label → catalog icon →
 * folder) from one implementation instead of two sets of matching classes that drift apart.
 *
 * The color token is applied as `color` only. Everything drawn here (the label text, and a lucide
 * icon whose strokes are `currentColor`) inherits it, while the button's background stays on the
 * theme's `appSidebar.itemActive` — tinting the background instead could sink the active-project
 * highlight below usable contrast in any of the 36 bundled themes.
 */
export const ProjectDisplayGlyph: FC<ProjectDisplayGlyphProps> = ({ display }) => {
    const style = display.colorVar === null ? undefined : { color: display.colorVar }
    const Icon = PROJECT_ICON_COMPONENT_MAP[resolveProjectIconName(display.icon)]

    if (display.label !== null)
        return (
            <span style={style} className={cn('font-semibold leading-none', resolveProjectLabelClassName(display.label))}>
                {display.label}
            </span>
        )

    return <Icon className='size-5' style={style} />
}
