import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { treeRowsQueryOptions, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenTab } from '@entities/layout/layout.query'
import { useCreateEntry } from '@entities/file/file.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
}

const PATH_SEPARATOR = '/'

const toFileTreeRow = (row: TreeRow): FileTreeRow => ({
    id: row.path,
    path: row.path,
    name: row.name,
    depth: row.depth,
    kind: row.kind === 'directory' ? 'directory' : 'file',
    expanded: row.expanded,
    gitStatus: null,
})

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

const joinPath = (dir: string, name: string) => `${dir.endsWith(PATH_SEPARATOR) ? dir.slice(0, -1) : dir}${PATH_SEPARATOR}${name}`

export const ExplorerContainer: FC<ExplorerContainerProps> = ({ projectId }) => {
    const [selectedRow, setSelectedRow] = useState<FileTreeRow | null>(null)

    const { data: page } = useQuery(treeRowsQueryOptions(projectId))
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { mutate: toggleNode, mutateAsync: toggleNodeAsync } = useToggleTreeNode(projectId)
    const { mutateAsync: refreshTreeDir } = useRefreshTreeDir(projectId)
    const { mutateAsync: revealTreeNode } = useRevealTreeNode(projectId)
    const { mutateAsync: createEntry } = useCreateEntry()
    const { mutate: openTab } = useOpenTab(projectId)

    const rows = (page?.rows ?? []).map(toFileTreeRow)

    const fileNameOf = (path: string) => path.slice(path.lastIndexOf(PATH_SEPARATOR) + 1)

    const targetDirFor = (row: FileTreeRow | null) => {
        if (row) return row.kind === 'directory' ? row.path : parentDirOf(row.path)
        return project?.root ?? null
    }

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

    const createTreeEntry = async (name: string, isDir: boolean) => {
        const targetDir = targetDirFor(selectedRow)
        if (!targetDir) return
        try {
            const path = joinPath(targetDir, name)
            await createEntry({ path, isDir })
            await revealTreeNode({ projectId, path })
        } catch (error) {
            if (error instanceof Error) toast.error(error.message)
        }
    }

    const collapseAllExpanded = async () => {
        const expandedDirPaths = rows.filter((row) => row.kind === 'directory' && row.expanded).map((row) => row.path)
        for (const path of expandedDirPaths) {
            await toggleNodeAsync({ projectId, path })
        }
    }

    const refreshVisibleTree = async () => {
        if (!project) return
        const expandedDirPaths = rows.filter((row) => row.kind === 'directory' && row.expanded).map((row) => row.path)
        for (const dir of [project.root, ...expandedDirPaths]) {
            await refreshTreeDir({ projectId, dir })
        }
    }

    return (
        <ExplorerPanel
            projectId={projectId}
            rows={rows}
            onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
            onOpenPreview={(row) => openFileTab(row, true)}
            onOpenPinned={(row) => openFileTab(row, false)}
            onSelectionChange={setSelectedRow}
            onOpenSearchMatch={openSearchMatch}
            onCreateFile={(name) => void createTreeEntry(name, false)}
            onCreateFolder={(name) => void createTreeEntry(name, true)}
            onRefresh={() => void refreshVisibleTree()}
            onCollapseAll={() => void collapseAllExpanded()}
        />
    )
}
