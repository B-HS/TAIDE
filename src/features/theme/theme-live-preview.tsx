import type { CSSProperties, FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { SyntaxStyle } from '@shared/api/bindings'
import { toCssVariables } from '@shared/lib/theme-variables'

type ThemeLivePreviewProps = {
    colors: Record<string, string>
    syntax: Record<string, SyntaxStyle>
}

const syntaxStyle = (style: SyntaxStyle | undefined): CSSProperties => ({
    color: style?.fg,
    fontWeight: style?.bold ? 700 : undefined,
    fontStyle: style?.italic ? 'italic' : undefined,
})

export const ThemeLivePreview: FC<ThemeLivePreviewProps> = ({ colors, syntax }) => {
    const { t } = useTranslation()
    const vars = toCssVariables(colors) as CSSProperties

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
            <div className='bg-editor-background flex flex-col gap-0.5 px-3 py-3 font-mono'>
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
                    <span style={syntaxStyle(syntax.comment)}>{'// ' + t('themeEditor.previewCommentText')}</span>
                </div>
                <div className='pl-4'>
                    <span style={syntaxStyle(syntax.keyword)}>return</span> <span style={syntaxStyle(syntax.number)}>42</span>
                </div>
                <div>{'}'}</div>
            </div>
            <div className='bg-terminal-background text-terminal-foreground flex flex-col gap-1 px-3 py-2 font-mono'>
                <div>
                    <span style={{ color: colors['statusIndicator.success'] }}>{t('themeEditor.previewTerminalPrompt')}</span>{' '}
                    {t('themeEditor.previewTerminalCommand')}
                </div>
                <div className='flex items-center gap-1'>
                    <span className='h-3.5 w-1.5' style={{ backgroundColor: colors['terminal.cursor'] }} />
                </div>
            </div>
        </div>
    )
}
