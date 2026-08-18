import path from 'node:path'
import { defineConfig, devices } from '@playwright/test'
import { E2E_TMP_DIR, STORAGE_STATE_PATH } from './lib/paths'

/**
 * This config deliberately has no `webServer` block. The harness never starts TAIDE itself — it
 * connects to the remote-control server of an app instance the user already launched with
 * `bun run tauri dev` (see docs/quality-assurance/2026-08-18-e2e-harness.md). Port discovery and
 * login happen in `globalSetup`; each spec reads the result through `lib/taide-fixture.ts`'s
 * `taideBaseUrl` fixture rather than Playwright's built-in `baseURL` option, since that option is
 * fixed at config-load time and cannot reflect a port discovered later.
 *
 * `outputDir` is pinned under `e2e/.tmp/` (already gitignored) rather than Playwright's default
 * `e2e/test-results/` — `trace: 'retain-on-failure'` bundles the authenticated remote session's DOM
 * snapshot, network log, and `taide_remote_session` cookie into `trace.zip`, so its directory must
 * never be a plausible `git add` target. `.gitignore` also lists `e2e/test-results/` directly as a
 * second line of defense in case this option ever moves.
 */
export default defineConfig({
    testDir: './specs',
    testMatch: '**/*.e2e.ts',
    fullyParallel: false,
    workers: 1,
    retries: 0,
    reporter: 'list',
    globalSetup: './global-setup.ts',
    globalTeardown: './global-teardown.ts',
    outputDir: path.join(E2E_TMP_DIR, 'test-results'),
    use: {
        storageState: STORAGE_STATE_PATH,
        trace: 'retain-on-failure',
    },
    projects: [{ name: 'webkit', use: { ...devices['Desktop Safari'] } }],
})
