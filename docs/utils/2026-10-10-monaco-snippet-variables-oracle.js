import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { URI } from '../../node_modules/monaco-editor/esm/vs/base/common/uri.js'
import { WorkspaceFolder } from '../../node_modules/monaco-editor/esm/vs/platform/workspace/common/workspace.js'
import {
    ModelBasedVariableResolver,
    TimeBasedVariableResolver,
    WorkspaceBasedVariableResolver,
} from '../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetVariables.js'

const ROOT = resolve(import.meta.dir, '../..')
const source = readFileSync(resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/standalone/browser/standaloneServices.js'), 'utf8')
const labelStart = source.indexOf('class StandaloneUriLabelService {')
const labelEnd = source.indexOf('let StandaloneContextViewService =', labelStart)
const workspaceStart = source.indexOf('class StandaloneWorkspaceContextService {')
const workspaceEnd = source.indexOf('function updateConfigurationService(', workspaceStart)
if (labelStart < 0 || labelEnd <= labelStart || workspaceStart < 0 || workspaceEnd <= workspaceStart) {
    throw new Error('Installed Monaco standalone source boundaries changed')
}
const labelService = new Function(source.slice(labelStart, labelEnd) + '\nreturn new StandaloneUriLabelService()')()
const workspaceService = new Function(
    'URI',
    'WorkspaceFolder',
    'STANDALONE_EDITOR_WORKSPACE_ID',
    source.slice(workspaceStart, workspaceEnd) + '\nreturn new StandaloneWorkspaceContextService()',
)(URI, WorkspaceFolder, 'fixture-id')
const MODEL_NAMES = ['TM_FILENAME', 'TM_FILENAME_BASE', 'TM_DIRECTORY', 'TM_DIRECTORY_BASE', 'TM_FILEPATH', 'RELATIVE_FILEPATH', 'UNKNOWN']
const TIME_NAMES = [
    'CURRENT_YEAR',
    'CURRENT_YEAR_SHORT',
    'CURRENT_MONTH',
    'CURRENT_DATE',
    'CURRENT_HOUR',
    'CURRENT_MINUTE',
    'CURRENT_SECOND',
    'CURRENT_MILLISECOND',
    'CURRENT_DAY_NAME',
    'CURRENT_DAY_NAME_SHORT',
    'CURRENT_MONTH_NAME',
    'CURRENT_MONTH_NAME_SHORT',
    'CURRENT_SECONDS_UNIX',
    'CURRENT_MILLISECONDS_UNIX',
    'CURRENT_TIMEZONE_OFFSET',
    'CURRENT_TIMEZONE_NAME',
    'UNKNOWN',
]
const uris = [
    ...['/', '/work/file.ts', '/work/.gitignore', '/work/name.tar.gz', '/work/한글.with space.rs', '/work/trailing.', '/name', '/work/'].map((path) =>
        URI.file(path),
    ),
    URI.parse('untitled:tab-1234'),
    URI.from({ scheme: 'inmemory', authority: 'model', path: '/1' }),
]
const models = uris.map((uri) => {
    const resolver = new ModelBasedVariableResolver(labelService, { uri })
    return { path: uri.fsPath, values: MODEL_NAMES.map((name) => ({ name, expected: resolver.resolve({ name }) ?? null })) }
})
const dates = [
    '2024-02-29T14:59:58.007Z',
    '2024-12-31T23:59:59.999Z',
    '1969-12-31T23:59:59.999Z',
    '2000-01-01T00:00:00.000Z',
    '2026-07-01T01:02:03.004Z',
]
const times = dates.map((date) => {
    const resolver = new TimeBasedVariableResolver()
    resolver._date = new Date(date)
    return {
        date,
        offset: -resolver._date.getTimezoneOffset() * 60,
        values: TIME_NAMES.map((name) => ({ name, expected: resolver.resolve({ name }) ?? null })),
    }
})
const workspace = new WorkspaceBasedVariableResolver(workspaceService)
const workspaceValues = ['WORKSPACE_NAME', 'WORKSPACE_FOLDER', 'UNKNOWN'].map((name) => ({ name, expected: workspace.resolve({ name }) ?? null }))
writeFileSync(
    resolve(ROOT, 'native/taide-native-app/src/fixtures/snippet-variables-reference.json'),
    JSON.stringify({ models, times, workspace: workspaceValues }, null, 4) + '\n',
)
process.stdout.write(
    JSON.stringify({ models: models.length * MODEL_NAMES.length, times: times.length * TIME_NAMES.length, workspace: workspaceValues.length }) + '\n',
)
process.exit(0)
