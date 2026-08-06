import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { treeRowsQueryOptions, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenTab } from '@entities/layout/layout.query'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
}

const toFileTreeRow = (row: TreeRow): FileTreeRow => ({
    id: row.path,
    path: row.path,
    name: row.name,
    depth: row.depth,
    kind: row.kind === 'directory' ? 'directory' : 'file',
    expanded: row.expanded,
    gitStatus: null,
})

export const ExplorerContainer: FC<ExplorerContainerProps> = ({ projectId }) => {
    const { data: page } = useQuery(treeRowsQueryOptions(projectId))
    const { mutate: toggleNode } = useToggleTreeNode(projectId)
    const { mutate: openTab } = useOpenTab(projectId)

    const rows = (page?.rows ?? []).map(toFileTreeRow)

    const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

    const openFileTab = (row: FileTreeRow, preview: boolean) => {
        if (row.kind === 'directory') return
        openTab(
            { projectId, kind: { kind: 'file', path: row.path }, title: row.name, target: null, preview },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const openSearchMatch = (path: string) =>
        openTab(
            { projectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )

    return (
        <ExplorerPanel
            projectId={projectId}
            rows={rows}
            onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
            onOpenPreview={(row) => openFileTab(row, true)}
            onOpenPinned={(row) => openFileTab(row, false)}
            onOpenSearchMatch={openSearchMatch}
        />
    )
}
