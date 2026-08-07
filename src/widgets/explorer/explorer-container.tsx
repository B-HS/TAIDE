import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeNodeKind, FileTreeRow } from '@features/explorer/file-tree-row'
import { validateEntryName } from '@shared/lib/entry-name'
import { treeRowsQueryOptions, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenTab } from '@entities/layout/layout.query'
import { useCreateEntry } from '@entities/file/file.query'
import { projectQueryOptions } from '@entities/project/project.query'
import type { FileTreeDraft } from '@widgets/explorer/file-tree'
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
    const { t } = useTranslation()
    const [selectedRow, setSelectedRow] = useState<FileTreeRow | null>(null)
    const [draft, setDraft] = useState<FileTreeDraft | null>(null)
    const [draftError, setDraftError] = useState<string | null>(null)
    const [selectPathRequest, setSelectPathRequest] = useState<string | null>(null)

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

    const startDraft = async (kind: FileTreeNodeKind) => {
        const targetDir = targetDirFor(selectedRow)
        if (!targetDir) return
        const targetRow = rows.find((row) => row.path === targetDir)
        if (targetRow && !targetRow.expanded) await toggleNodeAsync({ projectId, path: targetDir })
        setDraft({ kind, parentDir: targetDir })
        setDraftError(null)
    }

    const cancelDraft = () => {
        setDraft(null)
        setDraftError(null)
    }

    const commitDraft = async (name: string) => {
        if (!draft) return
        const trimmedName = name.trim()
        if (!trimmedName) {
            cancelDraft()
            return
        }

        const siblingNames = rows.filter((row) => parentDirOf(row.path) === draft.parentDir).map((row) => row.name)
        const errorKey = validateEntryName(trimmedName, siblingNames)
        if (errorKey) {
            setDraftError(t(errorKey, { name: trimmedName }))
            return
        }

        const path = joinPath(draft.parentDir, trimmedName)
        try {
            await createEntry({ path, isDir: draft.kind === 'directory' })
            await refreshTreeDir({ projectId, dir: draft.parentDir })
            await revealTreeNode({ projectId, path })
            if (draft.kind === 'file') {
                openFileTab({ id: path, path, name: fileNameOf(path), depth: 0, kind: 'file', expanded: false, gitStatus: null }, false)
            }
            setDraft(null)
            setDraftError(null)
            setSelectPathRequest(path)
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error)
            setDraftError(message)
            toast.error(message)
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
            draft={draft}
            draftError={draftError}
            selectPathRequest={selectPathRequest}
            onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
            onOpenPreview={(row) => openFileTab(row, true)}
            onOpenPinned={(row) => openFileTab(row, false)}
            onSelectionChange={setSelectedRow}
            onOpenSearchMatch={openSearchMatch}
            onNewFile={() => void startDraft('file')}
            onNewFolder={() => void startDraft('directory')}
            onRefresh={() => void refreshVisibleTree()}
            onCollapseAll={() => void collapseAllExpanded()}
            onDraftCommit={(name) => void commitDraft(name)}
            onDraftCancel={cancelDraft}
            onSelectPathRequestHandled={() => setSelectPathRequest(null)}
        />
    )
}
