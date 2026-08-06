import type { FC } from 'react'
import { resolveFolderIcon } from '@shared/lib/file-icon'
import { cn } from '@shared/lib/cn'
import { FILE_ICON_COMPONENT_MAP } from '@shared/icons/file-icon-registry'

type FolderTypeIconProps = {
    folderName: string
    expanded: boolean
    className?: string
}

export const FolderTypeIcon: FC<FolderTypeIconProps> = ({ folderName, expanded, className }) => {
    const spec = resolveFolderIcon(folderName, expanded)
    const Icon = FILE_ICON_COMPONENT_MAP[spec.icon]
    return <Icon className={cn(spec.colorClass, className)} />
}
