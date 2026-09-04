import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { KEY_CHORD } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { expect, test } from '../lib/taide-fixture'

const WELCOME_TAB_NAME = /Welcome/
const TERMINAL_TAB_NAME = /Terminal/
const MARKER_FILE_NAME = 'e2e-terminal-reattach.txt'
const MARKER_CONTENT = 'taide-e2e-reattach-marker'

type TerminalSession = { id: string; running: boolean }

/**
 * Coverage for audit §4-B A6 (terminal re-attach): a pane renders only its active tab
 * (`pane-node-view.tsx`), so switching tabs away from a terminal unmounts its `TerminalSession` and
 * disposes the xterm instance — coming back must re-attach to the *same* pty
 * (`pty_attach` + ring-buffer replay), never spawn a second shell behind the same tab. A regression
 * there leaves an orphaned shell alive, which is why the oracle is `terminal_sessions` counted
 * *after* the re-attached terminal has proven it still drives a live pty: a second spawn would have
 * had every chance to show up by then.
 *
 * The replayed scrollback itself is deliberately not asserted — with the WebGL renderer xterm draws
 * into a canvas and exposes no DOM text to read (harness doc §6), so the functional check is a
 * shell command sent *after* the re-attach, whose effect is observed on disk exactly as spec 08
 * does. Terminal mount failures are surfaced through `pageerror` collection for the same reason.
 */
test('터미널 탭을 떠났다 돌아와도 같은 세션에 재부착되고 새 셸이 생기지 않는다', async ({ page, fixtureProject }) => {
    const pageErrors: string[] = []
    page.on('pageerror', (error) => pageErrors.push(error.message))

    await page.keyboard.press(KEY_CHORD.NEW_TERMINAL)
    const terminals = page.locator('.xterm')
    try {
        await expect(terminals.first()).toBeVisible()
    } catch (mountError) {
        throw new Error(`터미널 마운트 실패 (WebGL 예외 가능성 — 하네스 문서 §6) — page errors: ${JSON.stringify(pageErrors)}`, { cause: mountError })
    }

    let spawnedSessionId = ''
    await expect(async () => {
        const sessions = await invokeIpc<TerminalSession[]>(page, 'terminal_sessions', { projectId: fixtureProject.projectId })
        expect(sessions).toHaveLength(1)
        spawnedSessionId = sessions[0].id
    }).toPass()

    await page.getByRole('tab', { name: WELCOME_TAB_NAME }).click()
    await expect(terminals).toHaveCount(0)

    await page.getByRole('tab', { name: TERMINAL_TAB_NAME }).click()
    await expect(terminals).toHaveCount(1)
    await expect(terminals.first()).toBeVisible()

    const markerPath = path.join(fixtureProject.rootDir, MARKER_FILE_NAME)
    await terminals.first().click()
    await page.evaluate((command) => navigator.clipboard.writeText(command), `echo ${MARKER_CONTENT} > ${JSON.stringify(markerPath)}`)
    await page.keyboard.press(KEY_CHORD.PASTE)
    await page.keyboard.press('Enter')

    try {
        await expect(async () => {
            expect(await readFile(markerPath, 'utf8')).toContain(MARKER_CONTENT)
        }).toPass()
    } catch (markerWaitError) {
        throw new Error(`재부착된 터미널이 명령을 받지 못했습니다 — page errors: ${JSON.stringify(pageErrors)}`, { cause: markerWaitError })
    }

    const sessionsAfterReattach = await invokeIpc<TerminalSession[]>(page, 'terminal_sessions', { projectId: fixtureProject.projectId })
    expect(sessionsAfterReattach).toHaveLength(1)
    expect(sessionsAfterReattach[0].id).toBe(spawnedSessionId)
})
