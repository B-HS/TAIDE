import { rm } from 'node:fs/promises'
import path from 'node:path'
import { invokeIpc } from '../lib/ipc'
import { openPalette } from '../lib/palette'
import { expect, test } from '../lib/taide-fixture'

const EPHEMERAL_FILE_RELATIVE_PATH = 'src/ephemeral.ts'
const EPHEMERAL_QUERY = 'ephemeral'
const FILE_NOT_FOUND_TOAST_PATTERN = /File not found/

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §A: a quick-open row that
 * went stale (the file vanished after the index was built) must fail loudly with the localized
 * `error.file.notFound` toast — `layout_open_tab` validates the path before creating a tab — and
 * must not open an empty editor tab.
 *
 * Ordering is what makes this deterministic. The palette is opened and the `ephemeral` row is
 * awaited *before* the file is deleted, so the row is guaranteed to be on screen when it is
 * clicked: the only thing that could remove it is the fs-watcher echo (300ms debounce + a full
 * `search_list_files` re-walk → `PROJECT_FILES` invalidation), which cannot complete between the
 * `rm` resolving and the immediate click. Deleting first and then opening the palette would race
 * that same echo and turn the spec into a coin flip.
 */
test('인덱스가 낡아 사라진 파일을 퀵오픈에서 열면 File not found 토스트가 뜨고 탭은 열리지 않는다', async ({ page, fixtureProject }) => {
    const input = await openPalette(page)
    await input.fill(EPHEMERAL_QUERY)
    const ephemeralOption = page.getByRole('option').filter({ hasText: EPHEMERAL_QUERY }).first()
    await ephemeralOption.waitFor({ state: 'visible' })

    await rm(path.join(fixtureProject.rootDir, EPHEMERAL_FILE_RELATIVE_PATH), { force: true })
    await ephemeralOption.click()

    await expect(page.getByText(FILE_NOT_FOUND_TOAST_PATTERN).first()).toBeVisible()
    await expect(page.getByRole('tab', { name: /ephemeral\.ts/ })).toHaveCount(0)

    const layout = await invokeIpc(page, 'layout_get', { projectId: fixtureProject.projectId })
    expect(JSON.stringify(layout)).not.toContain(EPHEMERAL_FILE_RELATIVE_PATH)
})
