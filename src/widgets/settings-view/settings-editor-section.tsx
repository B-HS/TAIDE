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
import { SwitchField } from '@features/settings/switch-field'
import { TextField } from '@features/settings/text-field'
import { formatEditorRulers, NO_EDITOR_RULERS, normalizeEditorRulersText, parseEditorRulers } from '@shared/lib/editor-rulers'
import {
    DEFAULT_EDITOR_CURSOR_BLINKING,
    DEFAULT_EDITOR_CURSOR_STYLE,
    DEFAULT_EDITOR_RENDER_WHITESPACE,
    DEFAULT_EDITOR_TAB_SIZE,
} from '@shared/constants/code-editor'
import { DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import type { Settings } from '@shared/api/bindings'

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
            <SwitchField
                label={t('settings.formatOnSave')}
                checked={settings.formatOnSave ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), formatOnSave: checked })}
            />
            <SwitchField
                label={t('settings.organizeImportsOnSave')}
                description={t('settings.organizeImportsOnSaveDescription')}
                checked={settings.organizeImportsOnSave ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), organizeImportsOnSave: checked })}
            />
            <SwitchField
                label={t('settings.fixAllOnSave')}
                description={t('settings.fixAllOnSaveDescription')}
                checked={settings.fixAllOnSave ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), fixAllOnSave: checked })}
            />
            <SwitchField
                label={t('settings.trimTrailingWhitespaceOnSave')}
                description={t('settings.trimTrailingWhitespaceOnSaveDescription')}
                checked={settings.trimTrailingWhitespaceOnSave ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), trimTrailingWhitespaceOnSave: checked })}
            />
            <SwitchField
                label={t('settings.insertFinalNewlineOnSave')}
                description={t('settings.insertFinalNewlineOnSaveDescription')}
                checked={settings.insertFinalNewlineOnSave ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), insertFinalNewlineOnSave: checked })}
            />
            <SwitchField
                label={t('settings.editorConfigEnabled')}
                description={t('settings.editorConfigEnabledDescription')}
                checked={settings.editorConfigEnabled ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorConfigEnabled: checked })}
            />
            <SwitchField
                label={t('settings.editorCodeLens')}
                description={t('settings.editorCodeLensDescription')}
                checked={settings.editorCodeLensEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorCodeLensEnabled: checked })}
            />
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
            <SwitchField
                label={t('settings.editorWordWrap')}
                checked={settings.editorWordWrap ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorWordWrap: checked })}
            />
            <SwitchField
                label={t('settings.editorLineNumbers')}
                checked={settings.editorLineNumbers ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorLineNumbers: checked })}
            />
            <NumericField
                label={t('settings.editorTabSize')}
                value={settings.editorTabSize ?? DEFAULT_EDITOR_TAB_SIZE}
                min={MIN_TAB_SIZE}
                max={MAX_TAB_SIZE}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorTabSize: value })}
            />
            <SwitchField
                label={t('settings.editorInsertSpaces')}
                checked={settings.editorInsertSpaces ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorInsertSpaces: checked })}
            />
            <SwitchField
                label={t('settings.editorDetectIndentation')}
                description={t('settings.editorDetectIndentationHint')}
                checked={settings.editorDetectIndentation ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorDetectIndentation: checked })}
            />
            <OptionPicker
                label={t('settings.editorRenderWhitespace')}
                options={EDITOR_RENDER_WHITESPACE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={settings.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE}
                onSelect={(editorRenderWhitespace) => updateSettings({ ...emptySettingsPatch(), editorRenderWhitespace })}
            />
            <SwitchField
                label={t('settings.editorBracketPairColorization')}
                checked={settings.editorBracketPairColorization ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorBracketPairColorization: checked })}
            />
            <SwitchField
                label={t('settings.editorBracketPairGuides')}
                description={t('settings.editorBracketPairGuidesDescription')}
                checked={settings.editorBracketPairGuides ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorBracketPairGuides: checked })}
            />
            <div className='flex flex-col gap-1'>
                <TextField
                    label={t('settings.editorRulers')}
                    value={formatEditorRulers(settings.editorRulers ?? NO_EDITOR_RULERS)}
                    placeholder={t('settings.editorRulersPlaceholder')}
                    normalize={normalizeEditorRulersText}
                    onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorRulers: parseEditorRulers(value) })}
                />
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.editorRulersHint')}</span>
            </div>
            <SwitchField
                label={t('settings.editorFontLigatures')}
                checked={settings.editorFontLigatures ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFontLigatures: checked })}
            />
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
            <SwitchField
                label={t('settings.editorCursorSmoothCaretAnimation')}
                description={t('settings.editorCursorSmoothCaretAnimationDescription')}
                checked={settings.editorCursorSmoothCaretAnimation ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorCursorSmoothCaretAnimation: checked })}
            />
            <SwitchField
                label={t('settings.editorScrollBeyondLastLine')}
                checked={settings.editorScrollBeyondLastLine ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorScrollBeyondLastLine: checked })}
            />
            <SwitchField
                label={t('settings.editorSmoothScrolling')}
                description={t('settings.editorSmoothScrollingDescription')}
                checked={settings.editorSmoothScrolling ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorSmoothScrolling: checked })}
            />
            <SwitchField
                label={t('settings.editorStickyScroll')}
                description={t('settings.editorStickyScrollDescription')}
                checked={settings.editorStickyScrollEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorStickyScrollEnabled: checked })}
            />
            <SwitchField
                label={t('settings.editorSemanticHighlighting')}
                description={t('settings.editorSemanticHighlightingDescription')}
                checked={settings.editorSemanticHighlighting ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorSemanticHighlighting: checked })}
            />
            <SwitchField
                label={t('settings.editorFormatOnType')}
                description={t('settings.editorFormatOnTypeDescription')}
                checked={settings.editorFormatOnType ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnType: checked })}
            />
            <SwitchField
                label={t('settings.editorFormatOnPaste')}
                description={t('settings.editorFormatOnPasteDescription')}
                checked={settings.editorFormatOnPaste ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnPaste: checked })}
            />
            <SwitchField
                label={t('settings.editorSuggestPreview')}
                description={t('settings.editorSuggestPreviewDescription')}
                checked={settings.editorSuggestPreview ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorSuggestPreview: checked })}
            />
            <SwitchField
                label={t('settings.emmetEnabled')}
                description={t('settings.emmetEnabledDescription')}
                checked={settings.emmetEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), emmetEnabled: checked })}
            />
            <SwitchField
                label={t('settings.editorDiffHideUnchangedRegions')}
                description={t('settings.editorDiffHideUnchangedRegionsDescription')}
                checked={settings.editorDiffHideUnchangedRegions ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorDiffHideUnchangedRegions: checked })}
            />
            <SwitchField
                label={t('settings.editorDiffShowMoves')}
                description={t('settings.editorDiffShowMovesDescription')}
                checked={settings.editorDiffShowMoves ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorDiffShowMoves: checked })}
            />
        </SettingsSection>
    )
}
