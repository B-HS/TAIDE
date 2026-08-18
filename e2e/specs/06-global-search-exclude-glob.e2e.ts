import { KEY_CHORD, SEARCH_SETTLE_TIMEOUT_MS } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { expect, test } from '../lib/taide-fixture'

/**
 * Precondition (실측 필요 시 최우선 확인 지점): `getByPlaceholder('Search', { exact: true })` assumes
 * the literal placeholder text `"Search"` is unique to the global search query input
 * (`search-panel.tsx`). Other i18n keys (`search.title`/`keymap.search`/`keymap.category.search`)
 * also resolve to the label "Search", but none of them are currently rendered as an input
 * placeholder — only `search-panel.tsx`'s query field is. If a future panel adds a second
 * `placeholder="Search"` input, this locator becomes ambiguous and this spec must be revisited.
 */
test('전역 검색 결과 클릭으로 파일이 열리고, excludeGlob 적용 후 결과가 줄어든다', async ({ page, fixtureProject }) => {
    await page.keyboard.press(KEY_CHORD.SEARCH_PANEL)

    const queryInput = page.getByPlaceholder('Search', { exact: true })
    await expect(queryInput).toBeVisible()
    await queryInput.fill('greet')
    await queryInput.press('Enter')

    const matchSummary = page.getByText(/results in \d+ files/)
    await expect(matchSummary).toContainText('results in 2 files', { timeout: SEARCH_SETTLE_TIMEOUT_MS })

    const firstMatch = page.locator('div[role="button"]', { hasText: 'greet' }).first()
    await expect(firstMatch).toBeVisible()
    await firstMatch.click()

    await expect(async () => {
        const layout = await invokeIpc(page, 'layout_get', { projectId: fixtureProject.projectId })
        expect(JSON.stringify(layout)).toContain(fixtureProject.rootDir)
    }).toPass()

    const excludeInput = page.getByPlaceholder(/Files to exclude/)
    await excludeInput.fill('**/other.ts')
    await queryInput.click()
    await queryInput.press('Enter')

    await expect(matchSummary).toContainText('results in 1 files', { timeout: SEARCH_SETTLE_TIMEOUT_MS })
})
