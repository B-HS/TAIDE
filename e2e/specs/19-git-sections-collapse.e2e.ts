import { appendFile } from 'node:fs/promises'
import path from 'node:path'
import type { Locator } from '@playwright/test'
import { GIT_STATUS_SETTLE_TIMEOUT_MS, KEY_CHORD } from '../lib/constants'
import { expect, test } from '../lib/taide-fixture'

const GIT_SECTION_HEADER_SELECTOR = '[data-git-section-header]'
const CHANGES_SECTION_TITLE_PATTERN = /^Changes/
const STASH_SECTION_TITLE_PATTERN = /^Stash/
const DIRTY_FILE_NAME = 'other.ts'

const isExpanded = async (header: Locator) => (await header.getAttribute('aria-expanded')) === 'true'

/**
 * Coverage for `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §E: every SCM section is
 * headed by the one `GitSectionHeader` (`role='button'`, `aria-expanded`, `data-git-section-header`,
 * `sticky top-0`), a click toggles the section, and the Stashes section is not rendered at all while
 * there are no stashes — previously it sat at the top of the list whenever there was anything to
 * stash, which is the ambiguity §E set out to remove.
 *
 * The header's accessible name is its title followed by the count badge's text (`Changes1`), so
 * titles are matched as a prefix against the header's own text; the anchored `^Changes` also keeps
 * `Staged Changes`/`Merge Changes` out. `^Stash` is checked on section headers only, because the
 * panel's header bar carries a `Stash changes` icon button that would match the same prefix.
 *
 * The collapse state lives in module-scope memory for the life of the app process
 * (`git-section-collapse-memory.ts`) and the harness shares one app instance across specs, so the
 * `finally` re-expands `Changes` even on failure — spec 07 asserts on rows inside that section.
 */
test('git 뷰의 Changes 섹션 헤더는 sticky 이고 클릭으로 접었다 펼 수 있으며, 스태시가 없으면 Stashes 섹션이 없다', async ({
    page,
    fixtureProject,
}) => {
    await appendFile(path.join(fixtureProject.rootDir, DIRTY_FILE_NAME), '\nexport const e2eSectionMarker = true\n', 'utf8')

    await page.keyboard.press(KEY_CHORD.GIT_PANEL)
    const changesHeader = page.locator(GIT_SECTION_HEADER_SELECTOR).filter({ hasText: CHANGES_SECTION_TITLE_PATTERN }).first()
    await expect(changesHeader).toBeVisible({ timeout: GIT_STATUS_SETTLE_TIMEOUT_MS })
    await expect(changesHeader).toHaveRole('button')
    await expect(changesHeader).toHaveCSS('position', 'sticky')

    const dirtyRow = page.getByRole('button').filter({ hasText: DIRTY_FILE_NAME }).first()
    try {
        if (!(await isExpanded(changesHeader))) await changesHeader.click()
        await expect(changesHeader).toHaveAttribute('aria-expanded', 'true')
        await expect(dirtyRow).toBeVisible()

        await changesHeader.click()
        await expect(changesHeader).toHaveAttribute('aria-expanded', 'false')
        await expect(dirtyRow).toHaveCount(0)

        await changesHeader.click()
        await expect(changesHeader).toHaveAttribute('aria-expanded', 'true')
        await expect(dirtyRow).toBeVisible()

        await expect(page.locator(GIT_SECTION_HEADER_SELECTOR).filter({ hasText: STASH_SECTION_TITLE_PATTERN })).toHaveCount(0)
    } finally {
        if (!(await isExpanded(changesHeader))) await changesHeader.click()
    }
})
