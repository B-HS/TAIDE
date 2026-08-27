import { test as base, expect, type Page } from '@playwright/test'
import { DEFAULT_TEST_LOCALE, LOGIN_ENDPOINT } from './constants'
import { closeFixtureProject, createFixtureProject, type FixtureProject } from './fixture-project'
import { describeLoginFailure, loginOnce } from './login'
import { readRuntimeInfo } from './runtime-info'
import { applySettingsOverride } from './settings'

type TaideFixtures = {
    taideBaseUrl: string
    fixtureProject: FixtureProject
}

const NO_RUNTIME_INFO_MESSAGE =
    'e2e/.auth/runtime.json 이 없습니다 — globalSetup 부트스트랩(포트 발견+로그인)이 완료되지 못했습니다. ' +
    '위 globalSetup 실패 로그를 먼저 확인하세요.'

const RELOGIN_FAILED_MESSAGE = '자동 재로그인 1회 시도 후에도 /__taide/login 에 머물러 있습니다 — 즉시 중단합니다(잠금 회피, 재시도는 1회만 허용).'

const isOnLoginPage = (page: Page) => new URL(page.url()).pathname === LOGIN_ENDPOINT

/**
 * `globalSetup`'s `storageState` cookie can expire mid-run (e.g. the app restarted between specs).
 * When the first navigation of a test lands on `/__taide/login` instead of the app shell, this
 * retries login **exactly once** through the same single-attempt `loginOnce` `globalSetup` uses,
 * then re-navigates — never more than once, to stay inside § 잠금 회피 규약 (repeated automated
 * retries risk tripping the 60-second lockout).
 */
const ensureLoggedIn = async (page: Page, taideBaseUrl: string) => {
    if (!isOnLoginPage(page)) return

    const outcome = await loginOnce(page.context().request, taideBaseUrl)
    if (outcome.kind !== 'success') throw new Error(describeLoginFailure(outcome))

    await page.goto(taideBaseUrl)
    if (isOnLoginPage(page)) throw new Error(RELOGIN_FAILED_MESSAGE)
}

/**
 * Every spec goes through this fixture set instead of `@playwright/test`'s bare `test` — it wires
 * up the app connection (`taideBaseUrl`, read from the file `globalSetup` wrote after a confirmed
 * port + successful login) and a fresh, isolated project per test (`fixtureProject`). Import `test`
 * and `expect` from here, never from `@playwright/test` directly.
 *
 * Playwright's fixture setup functions take `(dependencies, use)` positionally — the second
 * parameter is named `provideFixtureValue` here rather than Playwright's conventional `use`
 * purely to dodge `eslint-plugin-react-hooks`'s name-based heuristic, which otherwise misreads a
 * function literally called `use(...)` as the unrelated React 19 `use()` hook.
 *
 * Playwright validates at runtime that the first parameter is written as an object destructuring
 * pattern (it parses the function source) — a plain identifier aborts the whole run with "First
 * argument must use the object destructuring pattern". A fixture with no real dependencies must
 * therefore still destructure something (`{ browserName }` here); an empty `{}` would trip
 * eslint's `no-empty-pattern` instead.
 */
export const test = base.extend<TaideFixtures>({
    taideBaseUrl: async ({ browserName }, provideFixtureValue) => {
        void browserName
        const runtimeInfo = await readRuntimeInfo()
        if (!runtimeInfo) throw new Error(NO_RUNTIME_INFO_MESSAGE)
        await provideFixtureValue(runtimeInfo.baseURL)
    },
    page: async ({ page, taideBaseUrl }, provideFixtureValue) => {
        await page.goto(taideBaseUrl)
        await ensureLoggedIn(page, taideBaseUrl)
        await applySettingsOverride(page, { language: DEFAULT_TEST_LOCALE })
        await provideFixtureValue(page)
    },
    fixtureProject: async ({ page }, provideFixtureValue) => {
        const fixture = await createFixtureProject(page)
        await provideFixtureValue(fixture)
        await closeFixtureProject(page, fixture)
    },
})

export { expect }
