import { fileURLToPath } from 'node:url'
import path from 'node:path'

const LIB_DIR = path.dirname(fileURLToPath(import.meta.url))

export const E2E_ROOT_DIR = path.resolve(LIB_DIR, '..')
export const E2E_AUTH_DIR = path.join(E2E_ROOT_DIR, '.auth')
export const E2E_TMP_DIR = path.join(E2E_ROOT_DIR, '.tmp')

export const STORAGE_STATE_PATH = path.join(E2E_AUTH_DIR, 'state.json')
export const RUNTIME_INFO_PATH = path.join(E2E_AUTH_DIR, 'runtime.json')
export const SETTINGS_SNAPSHOT_PATH = path.join(E2E_AUTH_DIR, 'settings-snapshot.json')

export const REMOTE_LOG_PATH = path.join(process.env.HOME ?? '', 'Library/Logs/dev.taide.app/TAIDE.log')

export const MACOS_APP_SETTINGS_PATH = path.join(process.env.HOME ?? '', 'Library/Application Support/dev.taide.app/settings.json')
