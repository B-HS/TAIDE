import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { KEY_CHORD, TERMINAL_DECORATION_SOFT_CHECK_TIMEOUT_MS } from '../lib/constants'
import { expect, test } from '../lib/taide-fixture'

const MARKER_FILE_NAME = 'e2e-terminal-marker.txt'
const MARKER_CONTENT = 'taide-e2e-terminal-marker'

/**
 * Pilot risk #1 (design doc § 리스크 1, highest priority to measure): `terminal-view.tsx`'s mount
 * effect calls `new WebglAddon()` with no `try`/`catch` around it. If Playwright's headless WebKit
 * lacks WebGL2, that throw happens synchronously inside the effect and can abort the whole terminal
 * mount instead of just disabling the renderer. This spec collects `pageerror` events for the
 * duration of the terminal interaction specifically so a mount failure surfaces as a readable
 * diagnostic instead of a bare locator timeout.
 */
test('터미널에 명령을 입력하면 셸이 실행되고 OSC133 데코레이션이 나타난다', async ({ page, fixtureProject }) => {
    const pageErrors: string[] = []
    page.on('pageerror', (error) => pageErrors.push(error.message))

    await page.keyboard.press(KEY_CHORD.NEW_TERMINAL)

    const terminal = page.locator('.xterm').first()
    try {
        await expect(terminal).toBeVisible()
    } catch (mountError) {
        throw new Error(`터미널 마운트 실패 (WebGL 예외 가능성 — 설계 § 리스크 1) — page errors: ${JSON.stringify(pageErrors)}`, {
            cause: mountError,
        })
    }

    await terminal.click()
    const markerPath = path.join(fixtureProject.rootDir, MARKER_FILE_NAME)
    await page.keyboard.type(`echo ${MARKER_CONTENT} > ${JSON.stringify(markerPath)}`)
    await page.keyboard.press('Enter')

    try {
        await expect(async () => {
            const content = await readFile(markerPath, 'utf8')
            expect(content).toContain(MARKER_CONTENT)
        }).toPass()
    } catch (markerWaitError) {
        throw new Error(`마커 파일 대기 실패 — 셸이 명령을 받지 못했을 수 있음 — page errors: ${JSON.stringify(pageErrors)}`, {
            cause: markerWaitError,
        })
    }

    const anyDecoration = page.locator('.xterm-decoration-overview-ruler').or(page.locator('.xterm-decoration')).first()
    await expect.soft(anyDecoration).toBeVisible({ timeout: TERMINAL_DECORATION_SOFT_CHECK_TIMEOUT_MS })
})
