import { readFile } from 'node:fs/promises'
import { FALLBACK_EDITOR_FONT_SIZE, FONT_SIZE_SENTINEL_DELTA, KEY_CHORD } from '../lib/constants'
import { invokeIpc } from '../lib/ipc'
import { replaceEditorContentViaPaste } from '../lib/monaco-clipboard'
import { runPaletteCommand } from '../lib/palette'
import { MACOS_APP_SETTINGS_PATH } from '../lib/paths'
import { expect, test } from '../lib/taide-fixture'

const INVALID_JSON_TOAST_TEXT = 'settings.json contains invalid JSON'

test('settings.json AppFile 를 저장하면 settings_get·디스크에 반영되고, 잘못된 JSON 은 거부된다', async ({ page, fixtureProject }) => {
    void fixtureProject
    await runPaletteCommand(page, 'Open settings.json')

    const editor = page.locator('.monaco-editor').first()
    await expect(editor).toBeVisible()

    const baseline = await invokeIpc<{ editorFontSize?: number }>(page, 'settings_get')
    const sentinelFontSize = (baseline.editorFontSize ?? FALLBACK_EDITOR_FONT_SIZE) + FONT_SIZE_SENTINEL_DELTA
    const validPatch = { ...baseline, editorFontSize: sentinelFontSize }

    await replaceEditorContentViaPaste(page, editor, JSON.stringify(validPatch, null, 4))
    await page.keyboard.press(KEY_CHORD.SAVE)

    await expect(async () => {
        const settings = await invokeIpc<{ editorFontSize?: number }>(page, 'settings_get')
        expect(settings.editorFontSize).toBe(sentinelFontSize)
    }).toPass()

    await expect(async () => {
        const onDisk = JSON.parse(await readFile(MACOS_APP_SETTINGS_PATH, 'utf8')) as { editorFontSize?: number }
        expect(onDisk.editorFontSize).toBe(sentinelFontSize)
    }).toPass()

    await replaceEditorContentViaPaste(page, editor, '{ this is not valid json')
    await page.keyboard.press(KEY_CHORD.SAVE)

    await expect(page.getByText(INVALID_JSON_TOAST_TEXT)).toBeVisible()

    const onDiskAfterRejection = JSON.parse(await readFile(MACOS_APP_SETTINGS_PATH, 'utf8')) as { editorFontSize?: number }
    expect(onDiskAfterRejection.editorFontSize).toBe(sentinelFontSize)
})
