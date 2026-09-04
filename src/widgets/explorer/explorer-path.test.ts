import { describe, expect, test } from 'bun:test'
import { joinPath, parentDirOf } from '@widgets/explorer/explorer-path'

/**
 * The two path helpers behind explorer create/rename/paste. They deliberately do not touch the
 * filesystem, so what matters is the shapes they must never produce: a doubled separator (which
 * `ensure_within_root` on the Rust side treats as a different path) and an empty parent for a
 * top-level entry, which would send a create request with no directory at all.
 */
describe('parentDirOf', () => {
    test('중첩 경로는 마지막 세그먼트를 잘라낸다', () => {
        expect(parentDirOf('/project/src/app.tsx')).toBe('/project/src')
        expect(parentDirOf('/project/src')).toBe('/project')
    })

    test('루트 바로 아래 항목의 부모는 루트다 (빈 문자열이 아니다)', () => {
        expect(parentDirOf('/app.tsx')).toBe('/')
    })

    test('루트 자신의 부모는 루트다', () => {
        expect(parentDirOf('/')).toBe('/')
    })

    test('구분자가 없는 상대 이름의 부모도 루트로 수렴한다', () => {
        expect(parentDirOf('app.tsx')).toBe('/')
    })

    test('끝에 구분자가 붙은 경로는 빈 마지막 세그먼트를 잘라 디렉터리를 돌려준다', () => {
        expect(parentDirOf('/project/src/')).toBe('/project/src')
    })
})

describe('joinPath', () => {
    test('디렉터리와 이름 사이에 구분자를 하나 넣는다', () => {
        expect(joinPath('/project/src', 'app.tsx')).toBe('/project/src/app.tsx')
    })

    test('디렉터리가 구분자로 끝나도 구분자가 겹치지 않는다', () => {
        expect(joinPath('/project/src/', 'app.tsx')).toBe('/project/src/app.tsx')
    })

    test('루트 아래 항목은 선행 구분자 하나만 갖는다', () => {
        expect(joinPath('/', 'app.tsx')).toBe('/app.tsx')
    })

    test('이름이 비면 디렉터리 뒤에 구분자만 남는다 (호출부가 빈 이름을 먼저 막는다)', () => {
        expect(joinPath('/project', '')).toBe('/project/')
    })

    test('parentDirOf 로 얻은 부모와 다시 합치면 원래 경로가 된다', () => {
        const path = '/project/src/app.tsx'

        expect(joinPath(parentDirOf(path), 'app.tsx')).toBe(path)
    })
})
