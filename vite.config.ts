import { fileURLToPath } from 'node:url'
import babel from '@rolldown/plugin-babel'
import tailwindcss from '@tailwindcss/vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST
const resolvePath = (relative: string) => fileURLToPath(new URL(relative, import.meta.url))

/**
 * Opt-in dev-only switch for the Playwright e2e harness
 * (`docs/quality-assurance/2026-08-18-e2e-harness.md` §1·§2).
 *
 * The remote-control page is served in dev by proxying this dev server
 * (`domain/remote/serving.rs`, `proxy_dev`), so it receives Vite's injected HMR client but reaches
 * it under the *remote* origin — its WebSocket therefore targets the remote server, which speaks no
 * `vite-hmr` protocol, and the client falls into a 1s reconnect/ping loop that lasts the page's
 * whole lifetime. Across a long suite those pages accumulate in one WebKit process and the pilot's
 * later tests fail as a block (`2026-08-25-d39-e2e-pilot-run.md` C8). `server.ws: false` removes the
 * WebSocket server, so no client ever opens (or re-opens) that socket.
 *
 * Production bundles carry no HMR client, so this cannot affect a shipped build; it only trades hot
 * reload away for the one `bun run tauri dev` session the variable is exported into.
 */
const isHmrWebSocketDisabled = process.env.TAIDE_E2E_NO_HMR === '1'

const basePreset = reactCompilerPreset()
const compilerPreset = {
    ...basePreset,
    rolldown: {
        ...basePreset.rolldown,
        filter: {
            ...basePreset.rolldown.filter,
            id: { exclude: ['**/src/shared/api/bindings.ts', '**/*.worker.ts', '**/node_modules/**'] },
        },
    },
}

export default defineConfig({
    plugins: [react(), babel({ presets: [compilerPreset] }), tailwindcss()],
    resolve: {
        alias: {
            '@app': resolvePath('./src/app'),
            '@widgets': resolvePath('./src/widgets'),
            '@features': resolvePath('./src/features'),
            '@entities': resolvePath('./src/entities'),
            '@shared': resolvePath('./src/shared'),
        },
    },
    clearScreen: false,
    server: {
        port: 5173,
        strictPort: true,
        host: host || false,
        ws: isHmrWebSocketDisabled ? false : undefined,
        hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
        watch: { ignored: ['**/src-tauri/**'] },
    },
    envPrefix: ['VITE_', 'TAURI_ENV_*'],
    build: {
        target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
        minify: process.env.TAURI_ENV_DEBUG ? false : 'oxc',
        sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
    worker: { format: 'es' },
    optimizeDeps: { exclude: ['monaco-editor'] },
})
