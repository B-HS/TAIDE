import { execFile } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { promisify } from 'node:util'
import { buildChildProcessEnv } from './child-process-env'
import { HTTP_STATUS, LOGIN_ENDPOINT, LOGIN_PROBE_MARKER, PORT_DISCOVERY } from './constants'
import { REMOTE_LOG_PATH } from './paths'

const execFileAsync = promisify(execFile)

export type ProbeOutcome = 'confirmed' | 'not-ready' | 'no-match'

const isValidPort = (port: number) => Number.isInteger(port) && port >= PORT_DISCOVERY.MIN_PORT && port <= PORT_DISCOVERY.MAX_PORT

const readLastPortFromLog = async () => {
    const contents = await readFile(REMOTE_LOG_PATH, 'utf8').catch(() => null)
    if (contents === null) return null

    let lastMatch: RegExpExecArray | null = null
    for (const match of contents.matchAll(PORT_DISCOVERY.LOG_PORT_PATTERN)) lastMatch = match
    if (!lastMatch) return null

    const port = Number(lastMatch[1])
    return isValidPort(port) ? port : null
}

const runPgrep = async (args: string[]) => {
    const result = await execFileAsync('pgrep', args, { env: buildChildProcessEnv() }).catch((error: unknown) => {
        void error
        return null
    })
    if (!result) return []
    return result.stdout
        .split('\n')
        .map((line) => line.trim())
        .filter(Boolean)
}

const isBundledAppExecutable = async (pid: string) => {
    const result = await execFileAsync('ps', ['-p', pid, '-o', 'comm='], { env: buildChildProcessEnv() }).catch((error: unknown) => {
        void error
        return null
    })
    return result !== null && result.stdout.includes(PORT_DISCOVERY.APP_BUNDLE_EXECUTABLE_PATH_MARKER)
}

/**
 * Matches candidate pids two ways and unions the results: by process name (`-i`, cheap and precise
 * for the `taide` dev binary) and by full command line (`-i -f`, needed to catch a bundled
 * `TAIDE.app` whose process name/argv[0] may not literally be `taide` but whose executable path
 * under `TAIDE.app/Contents/MacOS/` does contain it) — then drops every pid whose own executable
 * path (`ps -o comm=`) resolves inside a `.app/Contents/MacOS/` bundle. d-49 gave the installed
 * build and a `bun run tauri dev` instance separate identifiers/data dirs so both can run at once,
 * but this harness drives the dev instance only (`e2e-harness.md` §0 — "하네스는 앱을 절대 기동하지
 * 않는다"); an installed build with REMOTE enabled would otherwise probe-confirm (stage 3 of
 * {@link discoverPort}) just as successfully as dev and get attached to instead. The dev binary
 * launched by `bun run tauri dev` never runs from inside a `.app` bundle, so this exclusion never
 * drops a legitimate dev candidate.
 */
const findCandidatePids = async () => {
    const [byProcessName, byCommandLine] = await Promise.all([
        runPgrep(['-i', PORT_DISCOVERY.PROCESS_NAME_PATTERN]),
        runPgrep(['-i', '-f', PORT_DISCOVERY.PROCESS_NAME_PATTERN]),
    ])
    const unionPids = [...new Set([...byProcessName, ...byCommandLine])]
    const isBundled = await Promise.all(unionPids.map(isBundledAppExecutable))
    return unionPids.filter((_, index) => !isBundled[index])
}

const LOOPBACK_LISTEN_PATTERN = /127\.0\.0\.1:(\d+)\s*\(LISTEN\)/g

const collectLoopbackListenPorts = async (pids: string[]) => {
    const ports = new Set<number>()
    for (const pid of pids) {
        const result = await execFileAsync('lsof', ['-nP', '-p', pid, '-iTCP', '-sTCP:LISTEN'], { env: buildChildProcessEnv() }).catch(
            (error: unknown) => {
                void error
                return null
            },
        )
        if (!result) continue
        for (const match of result.stdout.matchAll(LOOPBACK_LISTEN_PATTERN)) {
            const port = Number(match[1])
            if (isValidPort(port)) ports.add(port)
        }
    }
    return [...ports]
}

const probeLoginEndpoint = async (port: number): Promise<ProbeOutcome> => {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), PORT_DISCOVERY.PROBE_TIMEOUT_MS)
    try {
        const response = await fetch(`http://127.0.0.1:${port}${LOGIN_ENDPOINT}`, { signal: controller.signal, redirect: 'manual' })
        if (response.status >= HTTP_STATUS.REDIRECT_RANGE_START && response.status < HTTP_STATUS.REDIRECT_RANGE_END_EXCLUSIVE) return 'not-ready'
        if (!response.ok) return 'no-match'
        const body = await response.text()
        return body.includes(LOGIN_PROBE_MARKER) ? 'confirmed' : 'no-match'
    } catch (error) {
        void error
        return 'no-match'
    } finally {
        clearTimeout(timeout)
    }
}

const NO_APP_GUIDANCE =
    'TAIDE remote-control 서버를 찾지 못했습니다. `bun run tauri dev` 로 앱을 기동하고 ' +
    '설정 → REMOTE 에서 원격 접속을 활성화했는지 확인하세요 (docs/quality-assurance/2026-08-18-e2e-harness.md 준비 절차 참고).'

const NOT_READY_GUIDANCE =
    'TAIDE remote-control 서버는 응답했지만 로그인 준비가 되어 있지 않습니다. 설정 → REMOTE 에서 ' +
    '비밀번호를 설정하고 "비밀번호만으로 접속 허용" 을 켰는지 확인하세요.'

/**
 * Finds the loopback port the already-running app's remote-control server is bound to.
 *
 * Three-stage discovery, run in order until one candidate is confirmed by an HTTP probe:
 * 1. The last `원격 접속 서버 기동: port=<N>` line in the app's log file (subject to log rotation,
 *    so treated only as a priority candidate, not a final answer).
 * 2. Every loopback TCP LISTEN port owned by a `taide`-named process (`pgrep` + `lsof`), excluding
 *    any process running from inside a `.app/Contents/MacOS/` bundle (see
 *    {@link findCandidatePids}) — the app also runs other loopback servers (agent hooks, IDE
 *    bridge), and now potentially an installed build alongside dev (d-49), that must not be
 *    confused with the dev instance's remote-control.
 * 3. A `GET /__taide/login` probe against each candidate — only a 200 response containing the
 *    login form confirms the port. Never trusts stage 1/2 without this confirmation.
 *
 * Throws immediately (fail-fast) with user-facing guidance when no candidate probes as ready —
 * this function never starts the app itself.
 */
export const discoverPort = async () => {
    const candidates: number[] = []
    const logPort = await readLastPortFromLog()
    if (logPort !== null) candidates.push(logPort)

    const pids = await findCandidatePids()
    const lsofPorts = await collectLoopbackListenPorts(pids)
    for (const port of lsofPorts) if (!candidates.includes(port)) candidates.push(port)

    let sawNotReady = false
    for (const port of candidates) {
        const outcome = await probeLoginEndpoint(port)
        if (outcome === 'confirmed') return port
        if (outcome === 'not-ready') sawNotReady = true
    }

    throw new Error(sawNotReady ? NOT_READY_GUIDANCE : NO_APP_GUIDANCE)
}
