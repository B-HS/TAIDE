import DOMPurify from 'dompurify'
import { marked } from 'marked'

export const parseMarkdownToHtml = (source: string) => marked(source, { async: false })

export const renderMarkdownToSafeHtml = (source: string) => DOMPurify.sanitize(parseMarkdownToHtml(source))
