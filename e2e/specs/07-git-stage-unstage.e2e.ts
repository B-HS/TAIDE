import { appendFile } from 'node:fs/promises'
import path from 'node:path'
import { KEY_CHORD } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { expect, test } from '../lib/taide-fixture'

type StatusRow = { path: string; staged?: string | null; unstaged?: string | null }
type GitStatus = { rows: StatusRow[] }

const findRow = (status: GitStatus, fileName: string) => status.rows.find((row) => row.path.endsWith(fileName))

test('git 패널에서 stage 후 unstage 하면 상태가 각각 반영된다 (커밋 없음)', async ({ page, fixtureProject }) => {
    await appendFile(path.join(fixtureProject.rootDir, 'src/other.ts'), '\nexport const e2eDirtyMarker = true\n', 'utf8')

    await page.keyboard.press(KEY_CHORD.GIT_PANEL)
    const unstagedRow = page.getByRole('button').filter({ hasText: 'other.ts' }).first()
    await expect(unstagedRow).toBeVisible()

    await expect(async () => {
        const status = await invokeIpc<GitStatus>(page, 'git_status', { projectId: fixtureProject.projectId })
        expect(findRow(status, 'other.ts')?.unstaged).toBeTruthy()
    }).toPass()

    await unstagedRow.hover()
    await page.getByRole('button', { name: 'Stage Changes' }).click()

    await expect(async () => {
        const status = await invokeIpc<GitStatus>(page, 'git_status', { projectId: fixtureProject.projectId })
        const row = findRow(status, 'other.ts')
        expect(row?.staged).toBeTruthy()
        expect(row?.unstaged).toBeFalsy()
    }).toPass()

    const stagedRow = page.getByRole('button').filter({ hasText: 'other.ts' }).first()
    await stagedRow.hover()
    await page.getByRole('button', { name: 'Unstage Changes' }).click()

    await expect(async () => {
        const status = await invokeIpc<GitStatus>(page, 'git_status', { projectId: fixtureProject.projectId })
        const row = findRow(status, 'other.ts')
        expect(row?.unstaged).toBeTruthy()
        expect(row?.staged).toBeFalsy()
    }).toPass()
})
