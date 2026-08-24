import type { FC } from 'react'
import { buildFuzzyHighlightSegments } from '@shared/lib/fuzzy-match'

export const HighlightedText: FC<{ text: string; indices: number[] }> = ({ text, indices }) => (
    <>
        {buildFuzzyHighlightSegments(text, indices).map((segment, index) =>
            segment.matched ? (
                <mark key={index} className='bg-transparent text-panel-match-highlight font-semibold'>
                    {segment.text}
                </mark>
            ) : (
                segment.text
            ),
        )}
    </>
)
