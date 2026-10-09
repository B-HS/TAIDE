const SNAPSHOT_PATH = 'docs/quality-assurance/2026-10-09-native-function-audit.json'
const TABLE_PATH = 'docs/quality-assurance/2026-10-09-native-function-matrix.md'
const SOURCE_PREFIX = 'docs/quality-assurance/2026-10-06-native-audit-'
const DOMAINS = ['shell', 'editor', 'terminal-agent', 'explorer-search', 'preview', 'git', 'settings', 'commands-ui']
const STATUSES = ['complete', 'partial', 'unwired', 'missing', 'not-applicable', 'frozen']
const PIPE_PLACEHOLDER = '\u001f'
const TABLE_FIELDS = ['id', 'feature', 'ts', 'status', 'proof', 'note']
const PERCENT_SCALE = 100
const inputArguments = process.argv.slice(2)

if (inputArguments.some((argument) => argument !== '--write')) throw new Error('Only --write is supported')

const mapPlainText = (value, transform) =>
    value
        .split(/(`[^`]*`)/)
        .map((part) => (part.startsWith('`') ? part : transform(part)))
        .join('')

const cellsOf = (line) =>
    line
        .replaceAll('\\|', PIPE_PLACEHOLDER)
        .replace(/`[^`]*`/g, (value) => value.replaceAll('|', PIPE_PLACEHOLDER))
        .split('|')
        .slice(1, -1)
        .map((cell) => mapPlainText(cell.trim().replaceAll(PIPE_PLACEHOLDER, '|'), (part) => part.replaceAll('\\~', '~')))

const sourceRows = async (domain) => {
    const path = `${SOURCE_PREFIX}${domain}.md`
    const content = await Bun.file(path).text()
    let header = null
    let section = ''
    let count = 0
    return content.split('\n').flatMap((line, index) => {
        if (line.startsWith('### ')) section = line.slice('### '.length)
        if (!line.startsWith('|')) {
            header = null
            return []
        }
        const cells = cellsOf(line)
        if (cells.some((cell) => cell.startsWith('기능')) && cells.some((cell) => cell.includes('TS'))) {
            header = cells
            return []
        }
        if (!header || cells.every((cell) => /^:?-+:?$/.test(cell))) return []
        const feature = cells[header.findIndex((cell) => cell.startsWith('기능'))]
        const ts = cells[header.findIndex((cell) => cell.includes('TS'))]
        if (!feature || typeof ts !== 'string') throw new Error(`Invalid source row: ${path}:${index + 1}`)
        count += 1
        return [{ id: `${domain}-${count}`, domain, feature, ts, section, source: `${path}:${index + 1}` }]
    })
}

const snapshot = await Bun.file(SNAPSHOT_PATH).json()
if (!snapshot || typeof snapshot !== 'object' || !Array.isArray(snapshot.rows) || !snapshot.evidence || typeof snapshot.validation !== 'string') {
    throw new Error('Invalid audit snapshot')
}
if (snapshot.domainOrder?.join(',') !== DOMAINS.join(',')) throw new Error('Domain order mismatch')
const originals = (await Promise.all(DOMAINS.map(sourceRows))).flat()
const sourceById = new Map(originals.map((row) => [row.id, row]))
const rowsById = new Map()

for (const row of snapshot.rows) {
    if (!row || TABLE_FIELDS.some((field) => typeof row[field] !== 'string')) throw new Error('Invalid snapshot row')
    if (rowsById.has(row.id)) throw new Error(`Duplicate row: ${row.id}`)
    const original = sourceById.get(row.id)
    if (!original) throw new Error(`Unknown row: ${row.id}`)
    for (const field of ['domain', 'feature', 'ts', 'section', 'source']) {
        if (row[field] !== original[field]) throw new Error(`Source mismatch: ${row.id}.${field}`)
    }
    if (!STATUSES.includes(row.status) || !snapshot.evidence[row.proof]) throw new Error(`Invalid classification: ${row.id}`)
    rowsById.set(row.id, row)
}
if (rowsById.size !== sourceById.size) throw new Error('Missing requirement rows')

const paths = [...new Set(Object.values(snapshot.evidence).flatMap((proof) => [...proof.sources, ...proof.tests, ...proof.reports]))]
for (const path of paths) {
    if (typeof path !== 'string' || path.includes('..') || !['native/', 'crates/', 'docs/'].some((prefix) => path.startsWith(prefix))) {
        throw new Error('Invalid evidence path')
    }
    if (!(await Bun.file(path).exists())) throw new Error(`Missing evidence: ${path}`)
}

const countRows = (rows) => Object.fromEntries(STATUSES.map((status) => [status, rows.filter((row) => row.status === status).length]))
const counts = countRows(snapshot.rows)
const denominator = snapshot.rows.length - counts['not-applicable'] - counts.frozen
const percentage = ((counts.complete / denominator) * PERCENT_SCALE).toFixed(1)
const escapeCell = (value) =>
    mapPlainText(value, (part) => part.replaceAll('~', '\\~'))
        .replaceAll('|', '\\|')
        .replaceAll('\n', '<br>')
const link = (path) => `[${path}](../../${path})`
const domainTable = DOMAINS.map((domain) => {
    const rows = snapshot.rows.filter((row) => row.domain === domain)
    const count = countRows(rows)
    return `| ${domain} | ${rows.length} | ${STATUSES.map((status) => count[status]).join(' | ')} |`
}).join('\n')
const proofTable = Object.entries(snapshot.evidence)
    .map(
        ([id, proof]) =>
            `| ${id} | ${proof.sources.map(link).join('<br>')} | ${proof.tests.map(link).join('<br>')} | ${proof.reports.map(link).join('<br>')}<br>${escapeCell(proof.limitation)} |`,
    )
    .join('\n')
const details = DOMAINS.map((domain) => {
    const rows = snapshot.rows.filter((row) => row.domain === domain)
    const table = rows.map((row) => `| ${TABLE_FIELDS.map((field) => escapeCell(row[field])).join(' | ')} |`).join('\n')
    return `## ${domain}\n\nTS 경로의 상세 지도와 원본 행 위치는 [스냅샷](2026-10-09-native-function-audit.json)의 source 필드 및 [최초 감사](${SOURCE_PREFIX.replace('docs/quality-assurance/', '')}${domain}.md)에 있습니다. 최초 native 판정은 재사용하지 않습니다.\n\n| ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작 |\n| --- | --- | --- | --- | --- | --- |\n${table}`
}).join('\n\n')
const table = `# Rust-native 현재 기능 대응표\n\n기준: ${snapshot.date}, native 코드 ${snapshot.baseline}. [배치 14 감사](2026-10-09-native-batch14-function-audit.md)와 [전체 완료 근거](2026-10-09-native-completion-evidence.md)를 함께 읽습니다. 서브에이전트/workflow 없이 실제 Source·앱 도달·검증 경계를 대조했습니다.\n\ncomplete는 구현·실제 앱 경로·해당 동작 묶음의 자동 검사 근거가 있는 요구사항입니다. 전체 상태별 pixel·OS 입력·접근성·성능·출시 통과를 뜻하지 않습니다. partial은 일부 하위 동작/검증이 남은 항목, unwired는 모델/서비스/계약이 있으나 실제 소비자가 없는 항목, missing은 사용자 구현이 없는 항목입니다. not-applicable은 웹 기술/내부 지표 전용, frozen은 사용자 동결 범위입니다.\n\n같은 구현을 참조해도 화면/명령 진입·소비자가 다른 요구사항은 유지합니다. 이 표는 중복 참조를 포함한 복합 요구사항 행의 완료율이며 원자 기능 수나 배포까지 포함한 전체 전환율이 아닙니다.\n\n현재 ${snapshot.rows.length}행 중 웹 기술/내부 지표 전용 ${counts['not-applicable']}행과 동결 ${counts.frozen}행을 제외한 ${denominator}행이 기능 분모입니다. 완료 ${counts.complete}/${denominator}(${percentage}%), 부분 ${counts.partial}·미연결 ${counts.unwired}·미구현 ${counts.missing}입니다. 잔여 시간은 기능별 구현/실기/출시 규모와 실제 실행 시간 근거가 없어 미산정입니다.\n\n| 영역 | 행 | complete | partial | unwired | missing | not-applicable | frozen |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n${domainTable}\n| 합계 | ${snapshot.rows.length} | ${STATUSES.map((status) => counts[status]).join(' | ')} |\n\n## 현재 근거 지도\n\n테스트 열은 관련 동작 묶음의 검사 경로이며 모든 원본 분기/실기를 검사했다는 뜻이 아닙니다. ${snapshot.validation}\n\n| 묶음 | 현재 구현/앱 근거 | 관련 검사 경로 | 실행 근거/제한 |\n| --- | --- | --- | --- |\n${proofTable}\n\n${details}\n`

if (inputArguments.includes('--write')) await Bun.write(TABLE_PATH, table)
const rendered = (await Bun.file(TABLE_PATH).text())
    .split('\n')
    .filter((line) => line.startsWith('|'))
    .map(cellsOf)
    .filter((cells) => /^(shell|editor|terminal-agent|explorer-search|preview|git|settings|commands-ui)-\d+$/.test(cells[0] ?? ''))
if (rendered.length !== snapshot.rows.length) throw new Error('Rendered row count mismatch')
if (new Set(rendered.map((cells) => cells[0])).size !== rendered.length) throw new Error('Duplicate rendered rows')
for (const cells of rendered) {
    const row = rowsById.get(cells[0])
    if (!row || cells.length !== TABLE_FIELDS.length || TABLE_FIELDS.some((field, index) => cells[index] !== row[field])) {
        throw new Error(`Rendered row mismatch: ${cells[0]}`)
    }
}
console.info(
    JSON.stringify({ baseline: snapshot.baseline, rows: snapshot.rows.length, evidencePaths: paths.length, counts, denominator, percentage }),
)
