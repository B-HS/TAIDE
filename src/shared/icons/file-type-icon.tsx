import type { FC } from 'react'
import { resolveFileIcon } from '@shared/lib/file-icon'
import { cn } from '@shared/lib/cn'
import { FILE_ICON_COMPONENT_MAP } from '@shared/icons/file-icon-registry'

type FileTypeIconProps = {
    fileName: string
    className?: string
}

export const FileTypeIcon: FC<FileTypeIconProps> = ({ fileName, className }) => {
    const spec = resolveFileIcon(fileName)
    const Icon = FILE_ICON_COMPONENT_MAP[spec.icon]
    return <Icon className={cn(spec.colorClass, className)} />
}
