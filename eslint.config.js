import js from '@eslint/js'
import reactHooks from 'eslint-plugin-react-hooks'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const FSD_LAYERS = [
    { dir: 'shared', forbidden: ['@app', '@widgets', '@features', '@entities'] },
    { dir: 'entities', forbidden: ['@app', '@widgets', '@features'] },
    { dir: 'features', forbidden: ['@app', '@widgets'] },
    { dir: 'widgets', forbidden: ['@app'] },
]

export default tseslint.config(
    {
        ignores: [
            'dist/**',
            'dist-*/**',
            'target/**',
            'src-tauri/target/**',
            'src-tauri/gen/**',
            'src/shared/ui/**',
            'src/shared/api/bindings.ts',
            'docs/**',
        ],
    },
    js.configs.recommended,
    tseslint.configs.recommended,
    reactHooks.configs.flat['recommended-latest'],
    {
        files: ['**/*.{ts,tsx}'],
        languageOptions: {
            ecmaVersion: 2023,
            globals: { ...globals.browser, ...globals.es2023 },
            parserOptions: { ecmaFeatures: { jsx: true }, sourceType: 'module' },
        },
        rules: {
            '@typescript-eslint/no-explicit-any': 'error',
            '@typescript-eslint/consistent-type-imports': ['error', { prefer: 'type-imports' }],
            '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
            'no-restricted-syntax': [
                'error',
                { selector: 'TSEnumDeclaration', message: 'enum 대신 as const 객체 + union 타입을 사용한다 (common.md §5.5).' },
                { selector: 'FunctionDeclaration', message: 'function 키워드 대신 arrow function 을 사용한다 (common.md §3.1).' },
                {
                    selector: 'FunctionExpression[parent.type!="MethodDefinition"]',
                    message: 'function 키워드 대신 arrow function 을 사용한다 (common.md §3.1).',
                },
                { selector: "CallExpression[callee.name='useCallback']", message: 'useCallback 금지 — React Compiler 에 위임한다 (frontend.md §4).' },
                { selector: "CallExpression[callee.name='useMemo']", message: 'useMemo 금지 — React Compiler 에 위임한다 (frontend.md §4).' },
            ],
        },
    },
    ...FSD_LAYERS.map(({ dir, forbidden }) => ({
        files: [`src/${dir}/**/*.{ts,tsx}`],
        rules: {
            'no-restricted-imports': [
                'error',
                {
                    patterns: forbidden.map((layer) => ({
                        group: [`${layer}/*`, `${layer}`],
                        message: `FSD 의존 방향 위반 — ${dir} 는 ${layer.slice(1)} 를 import 할 수 없다 (fsd.md §2).`,
                    })),
                },
            ],
        },
    })),
    {
        files: ['e2e/**/*.ts'],
        languageOptions: {
            globals: { ...globals.node },
        },
    },
)
