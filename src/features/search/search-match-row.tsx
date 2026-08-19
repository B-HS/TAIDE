import type { FC } from 'react'
import type { SearchMatchRowData } from '@entities/search/search-result'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'

export type { SearchMatchRowData }

type SearchMatchRowProps = {
    match: SearchMatchRowData
    onClick: () => void
}

const MATCH_ROW_INDENT_PX = 32

const ContextLine: FC<{ line: number; text: string }> = ({ line, text }) => (
    <div className='text-app-sidebar-icon-default flex items-center gap-2 py-0.5 pr-2 opacity-70'>
        <span className='w-6 shrink-0 text-right tabular-nums'>{line}</span>
        <span className='truncate'>{text}</span>
    </div>
)

export const SearchMatchRow: FC<SearchMatchRowProps> = ({ match, onClick }) => {
    const beforeText = match.preview.slice(0, match.matchStart)
    const highlighted = match.preview.slice(match.matchStart, match.matchEnd)
    const afterText = match.preview.slice(match.matchEnd)

    return (
        <div
            role='button'
            tabIndex={0}
            onClick={onClick}
            onKeyDown={createActivationKeyDownHandler(onClick)}
            style={{ paddingLeft: MATCH_ROW_INDENT_PX }}
            className='hover:bg-explorer-item-hover flex cursor-default flex-col py-0.5 pr-2 text-xs select-none'>
            {match.before.map((text, index) => (
                <ContextLine key={`before-${index}`} line={match.line - match.before.length + index} text={text} />
            ))}
            <div className='flex items-center gap-2'>
                <span className='text-app-sidebar-icon-default w-6 shrink-0 text-right tabular-nums'>{match.line}</span>
                <span className='truncate'>
                    {beforeText}
                    <mark className='bg-panel-match-highlight text-app-background rounded-xs'>{highlighted}</mark>
                    {afterText}
                </span>
            </div>
            {match.after.map((text, index) => (
                <ContextLine key={`after-${index}`} line={match.line + index + 1} text={text} />
            ))}
        </div>
    )
}
