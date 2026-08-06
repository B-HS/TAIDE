import * as pdfjs from 'pdfjs-dist'
import PdfWorker from 'pdfjs-dist/build/pdf.worker.mjs?worker'

pdfjs.GlobalWorkerOptions.workerPort = new PdfWorker()

export { pdfjs }
