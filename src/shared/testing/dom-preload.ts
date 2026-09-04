/**
 * `bun test` preload (`bunfig.toml` `[test] preload`) that gives every test file a DOM.
 *
 * Bun runs this once per test process, before any test file is loaded, which is the only point
 * where `globalThis.window`/`document` can be installed early enough for `react-dom/client` and
 * `@testing-library/react` to work. Three run-wide contracts are established here:
 *
 * - **DOM globals**: happy-dom is registered at a fixed `http://localhost/` origin, so
 *   `location.search` is empty and `getWindowContext()` resolves to the main window — the default
 *   every test assumed while `window` did not exist at all. The user agent is pinned to the macOS
 *   WKWebView one because happy-dom derives `navigator.platform` from it (`Navigator.platform`
 *   reads the parenthesised part), and `IS_MAC` (`shared/constants/platform.ts`) decides which
 *   modifier key and which CLI commands exist. Without the override the suite would answer that
 *   question from whatever host it runs on — bun's own `navigator.platform` is `MacIntel`, an
 *   ubuntu CI runner's is not — and macOS-only catalog entries would vanish there.
 * - **`beforeEach(mock.restore)`**: bun does not isolate modules between test files, so spies
 *   created with `spyOn` are restored before every test instead of leaking into the next one.
 *   `mock.restore()` does NOT undo `mock.module()` — a module mock installed by one file stays
 *   installed for the rest of the process, so files keep calling `mock.module` in their own scope.
 * - **`afterEach(cleanup)`**: `@testing-library/react` only self-registers auto cleanup when
 *   `afterEach` is a runner global, which it never is under bun (hooks come from `bun:test`), so
 *   unmounting every mounted tree is registered explicitly here. It is pulled in with a dynamic
 *   `import()` *after* registration on purpose: `@testing-library/dom` binds `screen` to
 *   `document.body` while its module body runs, and a static import would run that before the DOM
 *   exists, leaving every `screen.getBy*` throwing "a global document has to be available".
 *
 * Coverage limits are documented in `docs/memory/test-conventions.md`: happy-dom is not WKWebView,
 * so webview-specific defects (IME `keyCode` 229, clipboard corruption) stay e2e-only.
 */
import { afterEach, beforeEach, mock } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

const TEST_DOCUMENT_URL = 'http://localhost/'
const MACOS_WEBVIEW_USER_AGENT =
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15'

if (!GlobalRegistrator.isRegistered) {
    GlobalRegistrator.register({ url: TEST_DOCUMENT_URL, settings: { navigator: { userAgent: MACOS_WEBVIEW_USER_AGENT } } })
}

const { cleanup } = await import('@testing-library/react')

beforeEach(() => mock.restore())
afterEach(() => cleanup())
