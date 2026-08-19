import type { CSSProperties, FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { SyntaxStyle } from '@shared/api/bindings'
import { TERMINAL_TOKENS } from '@entities/theme/theme-tokens'
import type { ThemeValues } from '@shared/lib/theme-draft'
import { toCssVariables } from '@shared/lib/theme-variables'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const TERMINAL_ANSI_TOKEN_COUNT = 16

type ThemeLivePreviewProps = {
    values: ThemeValues
}

const syntaxStyle = (style: SyntaxStyle | undefined): CSSProperties => ({
    color: style?.fg,
    fontWeight: style?.bold ? 700 : undefined,
    fontStyle: style?.italic ? 'italic' : undefined,
})

export const ThemeLivePreview: FC<ThemeLivePreviewProps> = ({ values }) => {
    const { t } = useTranslation()
    const { colors, syntax, terminal } = values
    const vars = toCssVariables(colors) as CSSProperties
    const ansiTokens = TERMINAL_TOKENS.slice(0, TERMINAL_ANSI_TOKEN_COUNT)

    return (
        <div style={vars} className='border-app-border flex flex-col overflow-hidden rounded-md border text-xs'>
            <div className='bg-tab-bar-background flex items-center gap-px px-1 pt-1'>
                <div
                    className='bg-tab-bar-tab-active-background text-tab-bar-tab-active-foreground border-tab-bar-tab-active-indicator flex items-center gap-1.5 rounded-t-sm border-t-2 px-2 py-1'
                    style={{ borderColor: colors['tabBar.tabActiveIndicator'] }}>
                    <span className='size-1.5 rounded-full' style={{ backgroundColor: colors['tabBar.dirtyDot'] }} />
                    <span>{t('themeEditor.previewEditorTab')}</span>
                </div>
                <div className='bg-tab-bar-tab-inactive-background text-tab-bar-tab-inactive-foreground rounded-t-sm px-2 py-1'>
                    {t('themeEditor.previewTerminalTab')}
                </div>
            </div>
            <div
                role='group'
                aria-label={t('themeEditor.previewSyntaxTitle')}
                className='bg-editor-background text-editor-foreground flex flex-col gap-0.5 px-3 py-3 font-mono'>
                <div>
                    <span style={syntaxStyle(syntax.keyword)}>const</span> <span style={syntaxStyle(syntax.variable)}>greeting</span> ={' '}
                    <span style={syntaxStyle(syntax.string)}>&apos;taide&apos;</span>
                </div>
                <div>
                    <span style={syntaxStyle(syntax.keyword)}>function</span> <span style={syntaxStyle(syntax.function)}>render</span>
                    <span style={syntaxStyle(syntax.punctuation)}>(</span>
                    <span style={syntaxStyle(syntax.parameter)}>props</span>
                    <span style={syntaxStyle(syntax.punctuation)}>)</span> {'{'}
                </div>
                <div className='pl-4'>
                    <span style={syntaxStyle(syntax.decorator)}>@decorator</span>
                </div>
                <div className='pl-4'>
                    <span style={syntaxStyle(syntax.comment)}>{'// ' + t('themeEditor.previewCommentText')}</span>
                </div>
                <div className='pl-4'>
                    <span style={syntaxStyle(syntax.keyword)}>return</span> <span style={syntaxStyle(syntax.number)}>42</span>
                </div>
                <div>{'}'}</div>
            </div>
            <div style={{ backgroundColor: terminal.background, color: terminal.foreground }} className='flex flex-col gap-2 px-3 py-2 font-mono'>
                <div>
                    <span style={{ color: colors['statusIndicator.success'] }}>{t('themeEditor.previewTerminalPrompt')}</span>{' '}
                    {t('themeEditor.previewTerminalCommand')}
                </div>
                <div className='flex items-center gap-1'>
                    <span className='h-3.5 w-1.5' style={{ backgroundColor: terminal.cursor }} />
                </div>
                <div role='group' aria-label={t('themeEditor.previewAnsiTitle')} className='grid grid-cols-8 gap-1'>
                    {ansiTokens.map((token) => (
                        <Tooltip key={token}>
                            <TooltipTrigger asChild>
                                <span className='size-3 rounded-sm' style={{ backgroundColor: terminal[token] }} />
                            </TooltipTrigger>
                            <TooltipContent side='top'>{token}</TooltipContent>
                        </Tooltip>
                    ))}
                </div>
            </div>
        </div>
    )
}
