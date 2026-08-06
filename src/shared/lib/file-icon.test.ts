import { describe, expect, test } from 'bun:test'
import { resolveFileIcon, resolveFolderIcon } from '@shared/lib/file-icon'

describe('resolveFileIcon', () => {
    test('ts/tsx 는 파란색 계열 아이콘을 반환한다', () => {
        expect(resolveFileIcon('index.ts')).toEqual({ icon: 'file-code', colorClass: 'text-status-info' })
        expect(resolveFileIcon('App.tsx')).toEqual({ icon: 'component', colorClass: 'text-status-info' })
    })

    test('js/jsx 는 노란색 계열 아이콘을 반환한다', () => {
        expect(resolveFileIcon('index.js')).toEqual({ icon: 'file-code', colorClass: 'text-status-warning' })
        expect(resolveFileIcon('App.jsx')).toEqual({ icon: 'component', colorClass: 'text-status-warning' })
    })

    test('json/md/css/html/rs/py/go/java/sh/yml/yaml/toml 확장자를 매칭한다', () => {
        expect(resolveFileIcon('data.json').icon).toBe('file-json')
        expect(resolveFileIcon('notes.md').icon).toBe('book-text')
        expect(resolveFileIcon('style.css').icon).toBe('palette')
        expect(resolveFileIcon('index.html').icon).toBe('globe')
        expect(resolveFileIcon('main.rs').icon).toBe('cog')
        expect(resolveFileIcon('script.py').icon).toBe('file-code')
        expect(resolveFileIcon('main.go').icon).toBe('file-code')
        expect(resolveFileIcon('Main.java').icon).toBe('coffee')
        expect(resolveFileIcon('build.sh').icon).toBe('terminal')
        expect(resolveFileIcon('ci.yml').icon).toBe('file-cog')
        expect(resolveFileIcon('ci.yaml').icon).toBe('file-cog')
        expect(resolveFileIcon('rustfmt.toml').icon).toBe('file-cog')
    })

    test('lock/이미지/pdf/zip 확장자를 매칭한다', () => {
        expect(resolveFileIcon('bun.lock').icon).toBe('lock')
        expect(resolveFileIcon('photo.png').icon).toBe('image')
        expect(resolveFileIcon('photo.jpg').icon).toBe('image')
        expect(resolveFileIcon('photo.jpeg').icon).toBe('image')
        expect(resolveFileIcon('anim.gif').icon).toBe('image')
        expect(resolveFileIcon('banner.webp').icon).toBe('image')
        expect(resolveFileIcon('icon.svg').icon).toBe('image')
        expect(resolveFileIcon('report.pdf').icon).toBe('file-text')
        expect(resolveFileIcon('archive.zip').icon).toBe('file-archive')
    })

    test('확장자 매칭은 대소문자를 구분하지 않는다', () => {
        expect(resolveFileIcon('INDEX.TS')).toEqual(resolveFileIcon('index.ts'))
        expect(resolveFileIcon('Data.JSON')).toEqual(resolveFileIcon('data.json'))
    })

    test('특수 파일명은 확장자 매칭보다 우선한다', () => {
        expect(resolveFileIcon('package.json')).toEqual({ icon: 'package', colorClass: 'text-status-error' })
        expect(resolveFileIcon('tsconfig.json')).toEqual({ icon: 'settings-2', colorClass: 'text-status-info' })
    })

    test('Cargo.toml/Cargo.lock 은 rust 전용 아이콘을 반환한다', () => {
        expect(resolveFileIcon('Cargo.toml')).toEqual({ icon: 'cog', colorClass: 'text-git-conflicted' })
        expect(resolveFileIcon('Cargo.lock')).toEqual({ icon: 'lock', colorClass: 'text-app-sidebar-icon-default' })
    })

    test('.gitignore 와 Dockerfile 은 특수 아이콘을 반환한다', () => {
        expect(resolveFileIcon('.gitignore')).toEqual({ icon: 'git-branch', colorClass: 'text-git-conflicted' })
        expect(resolveFileIcon('Dockerfile')).toEqual({ icon: 'container', colorClass: 'text-status-info' })
    })

    test('README/LICENSE 는 접두사로 매칭되어 확장자와 무관하게 동일한 아이콘을 반환한다', () => {
        expect(resolveFileIcon('README.md')).toEqual({ icon: 'book-marked', colorClass: 'text-status-info' })
        expect(resolveFileIcon('README')).toEqual({ icon: 'book-marked', colorClass: 'text-status-info' })
        expect(resolveFileIcon('readme.txt')).toEqual({ icon: 'book-marked', colorClass: 'text-status-info' })
        expect(resolveFileIcon('LICENSE')).toEqual({ icon: 'scale', colorClass: 'text-status-warning' })
        expect(resolveFileIcon('LICENSE.md')).toEqual({ icon: 'scale', colorClass: 'text-status-warning' })
    })

    test('.env 계열 파일은 확장자 파싱 없이 전용 아이콘을 반환한다', () => {
        expect(resolveFileIcon('.env')).toEqual({ icon: 'key', colorClass: 'text-status-warning' })
        expect(resolveFileIcon('.env.local')).toEqual({ icon: 'key', colorClass: 'text-status-warning' })
        expect(resolveFileIcon('.env.production')).toEqual({ icon: 'key', colorClass: 'text-status-warning' })
    })

    test('지원하지 않는 확장자는 기본 파일 아이콘으로 폴백한다', () => {
        expect(resolveFileIcon('archive.tar')).toEqual({ icon: 'file', colorClass: 'text-app-sidebar-icon-default' })
        expect(resolveFileIcon('unknown.xyz')).toEqual({ icon: 'file', colorClass: 'text-app-sidebar-icon-default' })
    })

    test('확장자가 없는 일반 파일명은 기본 파일 아이콘으로 폴백한다', () => {
        expect(resolveFileIcon('Makefile')).toEqual({ icon: 'file', colorClass: 'text-app-sidebar-icon-default' })
    })

    test('점으로 시작하되 특수 규칙에 없는 dotfile 은 확장자 없이 기본 아이콘으로 폴백한다', () => {
        expect(resolveFileIcon('.prettierrc')).toEqual({ icon: 'file', colorClass: 'text-app-sidebar-icon-default' })
    })
})

describe('resolveFolderIcon', () => {
    test('src/node_modules/dist/build/test/docs/public/assets 특수 폴더명을 매칭한다', () => {
        expect(resolveFolderIcon('src', false).icon).toBe('folder-code')
        expect(resolveFolderIcon('node_modules', false).icon).toBe('package')
        expect(resolveFolderIcon('dist', false).icon).toBe('box')
        expect(resolveFolderIcon('build', false).icon).toBe('box')
        expect(resolveFolderIcon('test', false).icon).toBe('flask-conical')
        expect(resolveFolderIcon('tests', false).icon).toBe('flask-conical')
        expect(resolveFolderIcon('docs', false).icon).toBe('book-marked')
        expect(resolveFolderIcon('public', false).icon).toBe('globe')
        expect(resolveFolderIcon('assets', false).icon).toBe('image')
    })

    test('.git 과 .github 는 서로 다른 아이콘을 반환한다', () => {
        const git = resolveFolderIcon('.git', false)
        const github = resolveFolderIcon('.github', false)
        expect(git.icon).toBe('git-branch')
        expect(github.icon).toBe('git-fork')
        expect(git).not.toEqual(github)
    })

    test('특수 폴더명은 대소문자를 구분하지 않는다', () => {
        expect(resolveFolderIcon('SRC', false)).toEqual(resolveFolderIcon('src', false))
        expect(resolveFolderIcon('Node_Modules', false)).toEqual(resolveFolderIcon('node_modules', false))
    })

    test('특수 폴더명은 expanded 여부와 무관하게 같은 아이콘을 유지한다', () => {
        expect(resolveFolderIcon('src', false)).toEqual(resolveFolderIcon('src', true))
    })

    test('일반 폴더명은 expanded 여부에 따라 다른 아이콘을 반환한다', () => {
        expect(resolveFolderIcon('components', false)).toEqual({ icon: 'folder', colorClass: 'text-app-sidebar-icon-default' })
        expect(resolveFolderIcon('components', true)).toEqual({ icon: 'folder-open', colorClass: 'text-app-sidebar-icon-default' })
    })
})
