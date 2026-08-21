import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { fontListQueryOptions } from '@entities/font/font.query'
import { FontPicker } from '@features/settings/font-picker'
import { NumericField } from '@features/settings/numeric-field'
import { OptionPicker } from '@features/settings/option-picker'
import { SettingsSection } from '@features/settings/settings-section'
import {
    DEFAULT_EDITOR_CURSOR_BLINKING,
    DEFAULT_EDITOR_CURSOR_STYLE,
    DEFAULT_EDITOR_RENDER_WHITESPACE,
    DEFAULT_EDITOR_TAB_SIZE,
} from '@shared/constants/code-editor'
import { DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import type { Settings } from '@shared/api/bindings'
import { Switch } from '@shared/ui/switch'

const MIN_AUTO_SAVE_DELAY_MS = 0
const MAX_AUTO_SAVE_DELAY_MS = 60_000
const DEFAULT_AUTO_SAVE_DELAY_MS = 0

const MIN_TAB_SIZE = 1
const MAX_TAB_SIZE = 8

const EDITOR_CURSOR_STYLE_OPTIONS = [
    { id: 'line', labelKey: 'settings.cursorStyleLine' },
    { id: 'block', labelKey: 'settings.cursorStyleBlock' },
    { id: 'underline', labelKey: 'settings.cursorStyleUnderline' },
] as const

const EDITOR_CURSOR_BLINKING_OPTIONS = [
    { id: 'blink', labelKey: 'settings.cursorBlinkingBlink' },
    { id: 'smooth', labelKey: 'settings.cursorBlinkingSmooth' },
    { id: 'phase', labelKey: 'settings.cursorBlinkingPhase' },
    { id: 'expand', labelKey: 'settings.cursorBlinkingExpand' },
    { id: 'solid', labelKey: 'settings.cursorBlinkingSolid' },
] as const

const EDITOR_RENDER_WHITESPACE_OPTIONS = [
    { id: 'none', labelKey: 'settings.renderWhitespaceNone' },
    { id: 'boundary', labelKey: 'settings.renderWhitespaceBoundary' },
    { id: 'selection', labelKey: 'settings.renderWhitespaceSelection' },
    { id: 'all', labelKey: 'settings.renderWhitespaceAll' },
] as const

type SettingsEditorSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
}

export const SettingsEditorSection: FC<SettingsEditorSectionProps> = ({ id, settings, updateSettings }) => {
    const { data: fonts = [], isPending: isFontsPending } = useQuery(fontListQueryOptions())

    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.editor')}>
            <NumericField
                label={t('settings.editorFontSize')}
                value={settings.editorFontSize ?? DEFAULT_CODE_FONT_SIZE}
                min={MIN_CODE_FONT_SIZE}
                max={MAX_CODE_FONT_SIZE}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorFontSize: value })}
            />
            {isFontsPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <FontPicker
                    label={t('settings.editorFontFamily')}
                    fonts={fonts}
                    value={settings.editorFontFamily ?? null}
                    onSelect={(editorFontFamily) => updateSettings({ ...emptySettingsPatch(), editorFontFamily: editorFontFamily ?? '' })}
                />
            )}
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.formatOnSave')}</span>
                <Switch
                    checked={settings.formatOnSave ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), formatOnSave: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.organizeImportsOnSave')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.organizeImportsOnSaveDescription')}</span>
                </span>
                <Switch
                    checked={settings.organizeImportsOnSave ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), organizeImportsOnSave: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.fixAllOnSave')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.fixAllOnSaveDescription')}</span>
                </span>
                <Switch
                    checked={settings.fixAllOnSave ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), fixAllOnSave: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorCodeLens')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorCodeLensDescription')}</span>
                </span>
                <Switch
                    checked={settings.editorCodeLensEnabled ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorCodeLensEnabled: checked })}
                />
            </label>
            <div className='flex flex-col gap-1'>
                <NumericField
                    label={t('settings.autoSaveDelayMs')}
                    value={settings.autoSaveDelayMs ?? DEFAULT_AUTO_SAVE_DELAY_MS}
                    min={MIN_AUTO_SAVE_DELAY_MS}
                    max={MAX_AUTO_SAVE_DELAY_MS}
                    onCommit={(value) => updateSettings({ ...emptySettingsPatch(), autoSaveDelayMs: value })}
                />
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.autoSaveDelayHint')}</span>
            </div>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorWordWrap')}</span>
                <Switch
                    checked={settings.editorWordWrap ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorWordWrap: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorLineNumbers')}</span>
                <Switch
                    checked={settings.editorLineNumbers ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorLineNumbers: checked })}
                />
            </label>
            <NumericField
                label={t('settings.editorTabSize')}
                value={settings.editorTabSize ?? DEFAULT_EDITOR_TAB_SIZE}
                min={MIN_TAB_SIZE}
                max={MAX_TAB_SIZE}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorTabSize: value })}
            />
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorInsertSpaces')}</span>
                <Switch
                    checked={settings.editorInsertSpaces ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorInsertSpaces: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorDetectIndentation')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorDetectIndentationHint')}</span>
                </span>
                <Switch
                    checked={settings.editorDetectIndentation ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorDetectIndentation: checked })}
                />
            </label>
            <OptionPicker
                label={t('settings.editorRenderWhitespace')}
                options={EDITOR_RENDER_WHITESPACE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={settings.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE}
                onSelect={(editorRenderWhitespace) => updateSettings({ ...emptySettingsPatch(), editorRenderWhitespace })}
            />
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorBracketPairColorization')}</span>
                <Switch
                    checked={settings.editorBracketPairColorization ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorBracketPairColorization: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorFontLigatures')}</span>
                <Switch
                    checked={settings.editorFontLigatures ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFontLigatures: checked })}
                />
            </label>
            <OptionPicker
                label={t('settings.editorCursorStyle')}
                options={EDITOR_CURSOR_STYLE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={settings.editorCursorStyle ?? DEFAULT_EDITOR_CURSOR_STYLE}
                onSelect={(editorCursorStyle) => updateSettings({ ...emptySettingsPatch(), editorCursorStyle })}
            />
            <OptionPicker
                label={t('settings.editorCursorBlinking')}
                options={EDITOR_CURSOR_BLINKING_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={settings.editorCursorBlinking ?? DEFAULT_EDITOR_CURSOR_BLINKING}
                onSelect={(editorCursorBlinking) => updateSettings({ ...emptySettingsPatch(), editorCursorBlinking })}
            />
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.editorScrollBeyondLastLine')}</span>
                <Switch
                    checked={settings.editorScrollBeyondLastLine ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorScrollBeyondLastLine: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorStickyScroll')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorStickyScrollDescription')}</span>
                </span>
                <Switch
                    checked={settings.editorStickyScrollEnabled ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorStickyScrollEnabled: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorSemanticHighlighting')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorSemanticHighlightingDescription')}</span>
                </span>
                <Switch
                    checked={settings.editorSemanticHighlighting ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorSemanticHighlighting: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorFormatOnType')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorFormatOnTypeDescription')}</span>
                </span>
                <Switch
                    checked={settings.editorFormatOnType ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnType: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.editorFormatOnPaste')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.editorFormatOnPasteDescription')}</span>
                </span>
                <Switch
                    checked={settings.editorFormatOnPaste ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnPaste: checked })}
                />
            </label>
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='flex flex-col gap-0.5'>
                    <span className='text-app-foreground'>{t('settings.emmetEnabled')}</span>
                    <span className='text-app-sidebar-icon-default'>{t('settings.emmetEnabledDescription')}</span>
                </span>
                <Switch
                    checked={settings.emmetEnabled ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), emmetEnabled: checked })}
                />
            </label>
        </SettingsSection>
    )
}
