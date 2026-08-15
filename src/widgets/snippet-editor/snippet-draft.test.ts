import { describe, expect, test } from 'bun:test'
import { isGlobalSnippetFileName } from '@shared/lib/snippet-file'
import {
    appendSnippetEntryDraft,
    buildLanguageSnippetFileName,
    createEmptySnippetEntryDraft,
    draftsToSnippetContent,
    hasDuplicateSnippetEntryNames,
    isSafeSnippetFileName,
    isSnippetEntryDraftValid,
    normalizeGlobalSnippetFileName,
    removeSnippetEntryDraft,
    snippetMapToDrafts,
    updateSnippetEntryDraft,
    type SnippetEntryDraft,
} from '@widgets/snippet-editor/snippet-draft'

describe('isGlobalSnippetFileName / buildLanguageSnippetFileName / normalizeGlobalSnippetFileName', () => {
    test('.code-snippets 확장자만 전역 파일로 인식한다', () => {
        expect(isGlobalSnippetFileName('global.code-snippets')).toBe(true)
        expect(isGlobalSnippetFileName('typescript.json')).toBe(false)
    })

    test('언어 파일명은 <languageId>.json 로 만든다', () => {
        expect(buildLanguageSnippetFileName('typescript')).toBe('typescript.json')
    })

    test('전역 파일명에 확장자가 없으면 붙이고, 있으면 그대로 둔다', () => {
        expect(normalizeGlobalSnippetFileName('my-snippets')).toBe('my-snippets.code-snippets')
        expect(normalizeGlobalSnippetFileName('my-snippets.code-snippets')).toBe('my-snippets.code-snippets')
        expect(normalizeGlobalSnippetFileName('  spaced  ')).toBe('spaced.code-snippets')
    })
})

describe('isSafeSnippetFileName', () => {
    test('경로 구분자·상위 디렉토리·콜론이 섞인 이름은 안전하지 않다', () => {
        expect(isSafeSnippetFileName('my-snippets.code-snippets')).toBe(true)
        expect(isSafeSnippetFileName('sub/evil.code-snippets')).toBe(false)
        expect(isSafeSnippetFileName('sub\\evil.code-snippets')).toBe(false)
        expect(isSafeSnippetFileName('../evil.code-snippets')).toBe(false)
        expect(isSafeSnippetFileName('C:evil.code-snippets')).toBe(false)
    })
})

describe('snippetMapToDrafts', () => {
    test('string 필드는 그대로, 배열 필드는 join 해서 draft 로 변환한다', () => {
        const drafts = snippetMapToDrafts({
            Main: { prefix: ['m', 'main'], body: ['fn main() {', '\t$0', '}'], description: 'entry point', scope: 'rust' },
        })
        expect(drafts).toHaveLength(1)
        expect(drafts[0]).toMatchObject({
            name: 'Main',
            prefix: 'm, main',
            body: 'fn main() {\n\t$0\n}',
            description: 'entry point',
            scope: 'rust',
        })
        expect(typeof drafts[0].id).toBe('string')
        expect(drafts[0].id.length).toBeGreaterThan(0)
    })

    test('description/scope 가 없으면 빈 문자열로 채운다', () => {
        const [draft] = snippetMapToDrafts({ Basic: { prefix: 'b', body: 'b' } })
        expect(draft.description).toBe('')
        expect(draft.scope).toBe('')
    })

    test('description 은 콤마로 분해하지 않고, 배열이면 개행으로 합친다 (prefix 와 다른 의미론)', () => {
        const [commaDraft] = snippetMapToDrafts({ Main: { prefix: 'm', body: 'm', description: 'Creates a loop, with an index' } })
        expect(commaDraft.description).toBe('Creates a loop, with an index')

        const [arrayDraft] = snippetMapToDrafts({ Main: { prefix: 'm', body: 'm', description: ['line1', 'line2'] } })
        expect(arrayDraft.description).toBe('line1\nline2')
    })
})

describe('append/update/remove SnippetEntryDraft', () => {
    test('append 은 빈 draft 를 끝에 추가한다', () => {
        const result = appendSnippetEntryDraft([])
        expect(result).toHaveLength(1)
        expect(result[0]).toMatchObject({ name: '', prefix: '', body: '' })
    })

    test('update 은 대상 id 의 필드만 바꾼다', () => {
        const draft = createEmptySnippetEntryDraft()
        const other = createEmptySnippetEntryDraft()
        const result = updateSnippetEntryDraft([draft, other], draft.id, { name: 'renamed' })
        expect(result.find((entry) => entry.id === draft.id)?.name).toBe('renamed')
        expect(result.find((entry) => entry.id === other.id)?.name).toBe('')
    })

    test('remove 는 대상 id 만 제거한다', () => {
        const draft = createEmptySnippetEntryDraft()
        const other = createEmptySnippetEntryDraft()
        const result = removeSnippetEntryDraft([draft, other], draft.id)
        expect(result).toEqual([other])
    })
})

describe('isSnippetEntryDraftValid', () => {
    test('name·prefix·body 가 모두 채워져야 유효하다', () => {
        expect(isSnippetEntryDraftValid({ id: '1', name: 'A', prefix: 'a', body: 'a', description: '', scope: '' })).toBe(true)
        expect(isSnippetEntryDraftValid({ id: '1', name: '', prefix: 'a', body: 'a', description: '', scope: '' })).toBe(false)
        expect(isSnippetEntryDraftValid({ id: '1', name: 'A', prefix: '  ', body: 'a', description: '', scope: '' })).toBe(false)
    })
})

describe('draftsToSnippetContent', () => {
    const baseDraft: SnippetEntryDraft = { id: '1', name: 'For', prefix: 'for', body: 'for (;;) {}', description: '', scope: '' }

    test('단일 값 필드는 문자열로, 여러 줄/콤마 값은 배열로 직렬화한다', () => {
        const content = draftsToSnippetContent([
            { ...baseDraft, prefix: 'for, fori', body: 'for (;;) {\n\t$0\n}', description: 'basic loop', scope: 'typescript,typescriptreact' },
        ])
        expect(JSON.parse(content)).toEqual({
            For: { prefix: ['for', 'fori'], body: ['for (;;) {', '\t$0', '}'], description: 'basic loop', scope: 'typescript,typescriptreact' },
        })
    })

    test('description/scope 가 비어 있으면 필드 자체를 생략한다', () => {
        const content = draftsToSnippetContent([baseDraft])
        expect(JSON.parse(content)).toEqual({ For: { prefix: 'for', body: 'for (;;) {}' } })
    })

    test('name·prefix·body 중 하나라도 비어 있는 draft 는 제외한다', () => {
        const content = draftsToSnippetContent([baseDraft, { ...baseDraft, id: '2', name: '' }])
        expect(Object.keys(JSON.parse(content))).toEqual(['For'])
    })

    test('description 에 콤마가 있어도 배열로 쪼개지 않고 문자열 그대로 저장한다', () => {
        const content = draftsToSnippetContent([{ ...baseDraft, description: 'Creates a loop, with an index' }])
        expect(JSON.parse(content).For.description).toBe('Creates a loop, with an index')
    })
})

describe('hasDuplicateSnippetEntryNames', () => {
    const baseDraft: SnippetEntryDraft = { id: '1', name: 'For', prefix: 'for', body: 'for (;;) {}', description: '', scope: '' }

    test('유효한 draft 중 이름이 겹치면 true 를 반환한다', () => {
        expect(hasDuplicateSnippetEntryNames([baseDraft, { ...baseDraft, id: '2' }])).toBe(true)
    })

    test('이름이 겹쳐도 무효한(빈 prefix/body) draft 는 계산에서 제외한다', () => {
        expect(hasDuplicateSnippetEntryNames([baseDraft, { ...baseDraft, id: '2', prefix: '', body: '' }])).toBe(false)
    })

    test('이름이 서로 다르면 false 를 반환한다', () => {
        expect(hasDuplicateSnippetEntryNames([baseDraft, { ...baseDraft, id: '2', name: 'While' }])).toBe(false)
    })
})
