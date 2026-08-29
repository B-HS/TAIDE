import * as pdfjs from 'pdfjs-dist'
import PdfWorker from 'pdfjs-dist/build/pdf.worker.mjs?worker'

let workerPort: Worker | null = null

/**
 * Returns the pdf.js namespace with its worker port installed, spawning that worker on the first
 * call instead of at module evaluation (audit §1-2). The previous top-level
 * `GlobalWorkerOptions.workerPort = new PdfWorker()` ran the moment anything in the static import
 * chain reached this module, so every launch started (and kept alive) a ~1.2MB worker thread even
 * for a session that never opens a PDF. `PdfPreview` is the only consumer and it is itself lazily
 * chunked, so the spawn now happens when a PDF is actually rendered.
 *
 * Idempotent — subsequent calls reuse the same port, which is what pdf.js expects: a `workerPort`
 * is a long-lived shared channel, not a per-document resource.
 */
export const getPdfjsWithWorker = () => {
    if (workerPort) return pdfjs

    workerPort = new PdfWorker()
    pdfjs.GlobalWorkerOptions.workerPort = workerPort
    return pdfjs
}
