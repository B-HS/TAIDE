import type { AppFileTarget } from '@shared/api/bindings'

const APP_FILE_MODEL_PATH_ROOT = '/__app-file__'

/**
 * Synthesizes a stable, unique monaco model path for an `AppFileTarget` — these tabs have no real
 * on-disk path exposed to the frontend (contract §3.3), but `CodeEditor`/`entities/editor/model-registry`
 * still need a path-shaped key to identify their model. Rooted under a directory name no real
 * project file can occupy, so it can never collide with an actual open file's model.
 */
export const resolveAppFileModelPath = (target: AppFileTarget) =>
    target.kind === 'settings' ? `${APP_FILE_MODEL_PATH_ROOT}/settings.json` : `${APP_FILE_MODEL_PATH_ROOT}/prompt/${target.id}.json`
