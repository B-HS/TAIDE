import { fileURLToPath } from 'node:url'
import babel from '@rolldown/plugin-babel'
import tailwindcss from '@tailwindcss/vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST
const resolvePath = (relative: string) => fileURLToPath(new URL(relative, import.meta.url))

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
