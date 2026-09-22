import { expect, mock, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import type { ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'

mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: (path: string) => ({ toString: () => path }) }, editor: {} } }))

const layoutFor = (path: string, dirty = false): ProjectLayout => ({
    version: 2,
    revision: 2,
    root: { node: 'leaf', id: 'pane', active: 'tab', tabs: [{ id: 'tab', kind: { kind: 'file', path }, title: path, dirty }] },
    focusedPane: 'pane',
})

const setup = async (dirty = false, modelContent: string | null = null) => {
    const { createWorkspaceResourceOperations } = await import('@entities/file/workspace-resource-operations')
    const calls: string[] = []
    const queryClient = new QueryClient()
    const operations = createWorkspaceResourceOperations(queryClient, {
        readModelContent: () => modelContent,
        listProjects: async () => [{ id: 'project', root: '/repo', name: 'repo' }],
        getLayout: async () => layoutFor('/repo/before.ts', dirty),
        listMirrors: async () => [],
        renameEntry: async () => {
            calls.push('disk rename')
            return null
        },
        deleteEntry: async () => {
            calls.push('disk delete')
            return null
        },
        followRenamedPathInTabs: async () => {
            expect(calls).toEqual(['disk rename'])
            calls.push('retarget and mirror')
            return { layout: layoutFor('/repo/after.ts', dirty), moved: [{ from: '/repo/before.ts', to: '/repo/after.ts', dirty }], closedPaths: [] }
        },
        followDeletedPathInTabs: async () => {
            calls.push('close tabs')
            return {
                layout: { ...layoutFor('/repo/after.ts'), root: { node: 'leaf', id: 'pane', active: null, tabs: [] } },
                moved: [],
                closedPaths: ['/repo/after.ts'],
            }
        },
    })
    return { operations, queryClient, calls }
}

test('LSP 개명은 디스크와 편집 상태를 옮긴 뒤 새 탭 경로를 공개한다', async () => {
    const { operations, queryClient, calls } = await setup(true)
    await operations.renameEntry({ from: '/repo/before.ts', to: '/repo/after.ts' })
    expect(calls).toEqual(['disk rename', 'retarget and mirror'])
    expect(queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL('project'))).toEqual(layoutFor('/repo/after.ts', true))
})

test('LSP 삭제는 저장하지 않은 탭이 있으면 디스크 변경 전에 거부한다', async () => {
    const { operations, calls } = await setup(true)
    await expect(operations.deleteEntry('/repo')).rejects.toThrow('unsaved')
    expect(calls).toEqual([])
})

test('dirty 표시 왕복 전인 편집 버퍼도 자동 삭제하지 않는다', async () => {
    const { operations, calls } = await setup(false, 'new draft')
    await expect(operations.deleteEntry('/repo/before.ts')).rejects.toThrow('unsaved')
    expect(calls).toEqual([])
})

test('LSP 삭제는 정상 파일의 열린 탭을 함께 닫는다', async () => {
    const { operations, queryClient, calls } = await setup()
    await operations.deleteEntry('/repo/after.ts')
    expect(calls).toEqual(['disk delete', 'close tabs'])
    expect(queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL('project'))?.root).toEqual({
        node: 'leaf',
        id: 'pane',
        active: null,
        tabs: [],
    })
})

test('프로젝트 밖 이동은 디스크를 바꾸기 전에 거부한다', async () => {
    const { operations, calls } = await setup()
    await expect(operations.renameEntry({ from: '/repo/before.ts', to: '/other/after.ts' })).rejects.toThrow('project roots')
    expect(calls).toEqual([])
})
