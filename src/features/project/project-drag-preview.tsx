import type { FC } from 'react'
import type { ProjectDisplayResolution } from '@shared/lib/project-display'
import { ProjectDisplayGlyph } from '@features/project/project-display-glyph'

type ProjectDragPreviewProps = {
    display: ProjectDisplayResolution
}

/**
 * What the pointer carries while a project is dragged from the rail onto a shell slot. The rail icon
 * itself stays where it is (dnd-kit leaves the drag source in place once an overlay exists) and fades
 * to a ghost, so this is the only thing that follows the cursor — which is also why it has to clear
 * the rail's own background rather than inherit it.
 *
 * Glyph only, no button: the real icon is still in the accessibility tree, and a second element
 * announcing the same project would be one the user can neither reach nor act on.
 */
export const ProjectDragPreview: FC<ProjectDragPreviewProps> = ({ display }) => (
    <div className='bg-app-sidebar-item-active text-app-foreground pointer-events-none flex size-10 items-center justify-center rounded-md opacity-90 shadow-lg'>
        <ProjectDisplayGlyph display={display} />
    </div>
)
