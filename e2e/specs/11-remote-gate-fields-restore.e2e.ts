import { FALLBACK_EDITOR_FONT_SIZE, FONT_SIZE_SENTINEL_DELTA, KEY_CHORD, REMOTE_GATED_SETTINGS_KEYS } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { runPaletteCommand } from '../lib/palette'
import { MACOS_APP_SETTINGS_PATH } from '../lib/paths'
import { expect, test } from '../lib/taide-fixture'

type GatedSettings = {
    shellOverride?: string | null
    remotePasswordOnlyLogin?: boolean
    remoteAllowedHosts?: string[]
    aiOmlxBaseUrl?: string | null
    editorFontSize?: number
}

type RemoteGatedSettingsKey = (typeof REMOTE_GATED_SETTINGS_KEYS)[number]

const GATE_ATTEMPT_SHELL_OVERRIDE = '/bin/taide-e2e-attempted-shell-override'
const GATE_ATTEMPT_REMOTE_ALLOWED_HOSTS = ['evil-e2e-attempt.invalid:9999']
const GATE_ATTEMPT_AI_OMLX_BASE_URL = 'http://taide-e2e-attempted.invalid:9999'

/**
 * One attempted value per gated field, ordered to match {@link REMOTE_GATED_SETTINGS_KEYS} — least
 * harmful first. `shellOverride` targets a path that does not exist on disk, so even an unstripped
 * write only makes the next terminal spawn fail to find a shell rather than run an attacker binary.
 * `aiOmlxBaseUrl` targets an `.invalid` TLD host for the same reason — even unstripped, no request
 * ever leaves the machine successfully.
 */
const GATE_ATTEMPT_VALUE_BUILDERS: { [K in RemoteGatedSettingsKey]: (baseline: GatedSettings) => GatedSettings[K] } = {
    shellOverride: () => GATE_ATTEMPT_SHELL_OVERRIDE,
    remotePasswordOnlyLogin: (baseline) => !baseline.remotePasswordOnlyLogin,
    remoteAllowedHosts: () => [...GATE_ATTEMPT_REMOTE_ALLOWED_HOSTS],
    aiOmlxBaseUrl: () => GATE_ATTEMPT_AI_OMLX_BASE_URL,
}

const REMOTE_GATE_UI_LOCATION: Record<RemoteGatedSettingsKey, string> = {
    shellOverride: 'Settings 탭 → Terminal 섹션 → Shell 필드',
    remotePasswordOnlyLogin: 'Settings 탭 → Remote 섹션 → "비밀번호만으로 접속 허용" 토글',
    remoteAllowedHosts: 'Settings 탭 → Remote 섹션 → 허용된 호스트 목록',
    aiOmlxBaseUrl: 'Settings 탭 → AI 섹션 → oMLX 행의 "기본 URL" 필드',
}

/**
 * Builds the hard-stop failure message for a confirmed gate regression. `global-teardown.ts`'s
 * restore write cannot fix this field — it goes through the exact same remote `settings_update`
 * dispatch (`strip_remote_gated_settings_patch`, `dispatch.rs`) that unconditionally nulls all four
 * gated fields out of *every* patch it forwards, restore attempts included. Recovery is manual only.
 */
const buildManualRecoveryMessage = (key: RemoteGatedSettingsKey, originalValue: unknown, regressedValue: unknown) =>
    [
        `게이트 회귀 감지 — 원격 세션의 settings.json 저장이 게이트 필드 "${key}" 를 실제로 바꿨습니다.`,
        'teardown 의 settings_update 복원도 이 필드를 항상 제거하는 동일한 원격 dispatch(strip_remote_gated_settings_patch, ' +
            'src-tauri/src/domain/remote/dispatch.rs)를 거치므로 자동 복원이 불가능합니다 — 수동 복구가 필요합니다.',
        `1. 앱 설정 UI: ${REMOTE_GATE_UI_LOCATION[key]} 를 아래 원래 값으로 되돌리세요.`,
        `2. 또는 ${MACOS_APP_SETTINGS_PATH} 파일을 직접 열어 "${key}": ${JSON.stringify(originalValue)} 로 고친 뒤 앱을 재시작하세요.`,
        `원래 값: ${JSON.stringify(originalValue)}`,
        `현재 반영된(회귀) 값: ${JSON.stringify(regressedValue)}`,
    ].join('\n')

/**
 * The one assertion only a remote (경로 A) session can make: `app_file_write`'s
 * `strip_remote_gated_settings` (dispatch.rs) must silently drop any attempted change to
 * remote-security-gated fields when the writer is a remote session, even though the surrounding
 * write otherwise succeeds (proven here by `editorFontSize`, an ungated field in the same patch,
 * actually changing on every iteration). A desktop session has no such gate, so this can't be
 * exercised there.
 *
 * Attempts are made **one gated field at a time, verified immediately** rather than as a single
 * combined patch: if the gate has actually regressed for one field, the loop hard-stops (throws)
 * before attempting the next field, so a caught regression cannot compound into a second or third
 * unrecoverable field write. See {@link buildManualRecoveryMessage} for why "unrecoverable" is
 * literal here — this is documented as a known, structural gap in
 * `docs/quality-assurance/2026-08-18-e2e-harness.md` §3.2.
 */
test('원격 세션에서 settings.json 을 통해 원격 게이트 필드를 바꾸려 해도 원값이 유지된다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await runPaletteCommand(page, 'Open settings.json')

    const editor = page.locator('.monaco-editor').first()
    await expect(editor).toBeVisible()

    const baseline = await invokeIpc<GatedSettings>(page, 'settings_get')

    for (const [index, key] of REMOTE_GATED_SETTINGS_KEYS.entries()) {
        const attemptValue = GATE_ATTEMPT_VALUE_BUILDERS[key](baseline)
        const sentinelFontSize = (baseline.editorFontSize ?? FALLBACK_EDITOR_FONT_SIZE) + FONT_SIZE_SENTINEL_DELTA * (index + 1)
        const attemptPatch = { ...baseline, [key]: attemptValue, editorFontSize: sentinelFontSize }

        await editor.click()
        await page.keyboard.press(KEY_CHORD.SELECT_ALL)
        await page.keyboard.insertText(JSON.stringify(attemptPatch, null, 4))
        await page.keyboard.press(KEY_CHORD.SAVE)

        await expect(async () => {
            const afterSave = await invokeIpc<GatedSettings>(page, 'settings_get')
            expect(afterSave.editorFontSize).toBe(sentinelFontSize)
        }).toPass()

        const afterSave = await invokeIpc<GatedSettings>(page, 'settings_get')
        if (JSON.stringify(afterSave[key]) !== JSON.stringify(baseline[key])) {
            throw new Error(buildManualRecoveryMessage(key, baseline[key], afterSave[key]))
        }
    }
})
