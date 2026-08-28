import { fileURLToPath } from 'node:url'
import os from 'node:os'
import path from 'node:path'

const LIB_DIR = path.dirname(fileURLToPath(import.meta.url))

export const E2E_ROOT_DIR = path.resolve(LIB_DIR, '..')
export const E2E_AUTH_DIR = path.join(E2E_ROOT_DIR, '.auth')
export const E2E_TMP_DIR = path.join(E2E_ROOT_DIR, '.tmp')

/**
 * Fixture projects live OUTSIDE the repository tree — never under `e2e/.tmp/`. Every fixture
 * contains a `tsconfig.json` (vtsls needs one to index the project, spec 05), and the Vite dev
 * server special-cases any `tsconfig*.json` appearing under its watch root: it clears its tsconfck
 * cache and **force-full-reloads every connected client**, including the remote-session page a
 * spec is driving. `vite.config.ts` only excludes `src-tauri/` from the watcher, so a fixture
 * created under `e2e/.tmp/` reloaded the page mid-test — this was the actual mechanism behind the
 * pilot's "remote webview renavigation storm" (candidate C1,
 * `docs/quality-assurance/2026-08-25-d39-e2e-pilot-run.md` §0: every `page.getByRole('dialog')`
 * that died with "navigated to <baseURL>" traces back to a fixture `tsconfig.json` write). Test
 * artifacts (`E2E_TMP_DIR`, `E2E_AUTH_DIR`) stay in-repo — none of their file names are
 * watcher-special, so they cannot trigger a reload.
 *
 * Not `os.tmpdir()` either: macOS FSEvents does not deliver events for the per-user temp tree
 * (`/var/folders/…`, canonical `/private/var/folders/…`) — reproduced 2026-08-27 with fixtures
 * there (both spellings): spec 07's external `appendFile` never surfaced in the git panel ("No
 * changes" through the full 15s settle timeout, 3/3 deterministic), because the app's fs watcher
 * simply never received the change. A home-directory location is fully FSEvents-monitorable and
 * still outside the Vite root; `Library/Caches/net.gumyo.taide.dev/` co-locates the throwaway
 * trees with the app's own cache namespace (this harness is already macOS-only — see
 * {@link REMOTE_LOG_PATH}).
 *
 * The identifier is `net.gumyo.taide.dev`, not the installed app's `net.gumyo.taide` (contract
 * d-49, renamed from `dev.taide.app`/`dev.taide.app.dev` under contract
 * 2026-08-29-bundle-identifier-rename): the harness only ever drives a `bun run tauri dev`
 * instance (`e2e-harness.md` §0 — "하네스는 앱을 절대 기동하지 않는다"), and `scripts/tauri.ts`
 * injects the `net.gumyo.taide.dev` identifier overlay (`src-tauri/tauri.dev.conf.json`) for
 * exactly that subcommand so its `app_data_dir`/log dir never collides with an installed release
 * build's.
 */
export const E2E_FIXTURE_PROJECTS_DIR = path.join(os.homedir(), 'Library/Caches/net.gumyo.taide.dev/e2e-fixtures')

export const STORAGE_STATE_PATH = path.join(E2E_AUTH_DIR, 'state.json')
export const RUNTIME_INFO_PATH = path.join(E2E_AUTH_DIR, 'runtime.json')
export const SETTINGS_SNAPSHOT_PATH = path.join(E2E_AUTH_DIR, 'settings-snapshot.json')

export const REMOTE_LOG_PATH = path.join(process.env.HOME ?? '', 'Library/Logs/net.gumyo.taide.dev/TAIDE.log')

export const MACOS_APP_SETTINGS_PATH = path.join(process.env.HOME ?? '', 'Library/Application Support/net.gumyo.taide.dev/settings.json')
