import type { FC } from 'react'

export type SearchMatchRowData = {
    line: number
    column: number
    preview: string
    matchStart: number
    matchEnd: number
}

type SearchMatchRowProps = {
    match: SearchMatchRowData
    onClick: () => void
}

const MATCH_ROW_INDENT_PX = 32

export const SearchMatchRow: FC<SearchMatchRowProps> = ({ match, onClick }) => {
    const before = match.preview.slice(0, match.matchStart)
    const highlighted = match.preview.slice(match.matchStart, match.matchEnd)
    const after = match.preview.slice(match.matchEnd)

    return (
        <div
            role='button'
            tabIndex={0}
            onClick={onClick}
            onKeyDown={(event) => event.key === 'Enter' && onClick()}
            style={{ paddingLeft: MATCH_ROW_INDENT_PX }}
            className='hover:bg-explorer-item-hover flex cursor-default items-center gap-2 py-0.5 pr-2 text-xs select-none'>
            <span className='text-app-sidebar-icon-default w-6 shrink-0 text-right tabular-nums'>{match.line}</span>
            <span className='truncate'>
                {before}
                <mark className='bg-panel-match-highlight text-app-background rounded-xs'>{highlighted}</mark>
                {after}
            </span>
        </div>
    )
}
