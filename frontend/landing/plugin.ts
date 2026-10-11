import { readFile } from 'node:fs/promises'
import type { IncomingMessage, ServerResponse } from 'node:http'
import path from 'node:path'

import type { Plugin, Rolldown } from 'vite'

import { type Locale, renderPage } from './page.ts'

/** The page's script; it brings the styles and fonts with it. */
const ENTRY = 'landing/main.ts'
/** Where each language's page is written. `/` serves home.html (public/_redirects). */
const FILES: Record<Locale, string> = { uk: 'home.html', en: 'en.html' }
/** Fonts the first screen needs in each language, fetched before the CSS asks. */
const FONTS: Record<Locale, RegExp> = {
  uk: /\/(unbounded-(latin|cyrillic)-600|manrope-(latin|cyrillic)-wght)-normal-[\w-]+\.woff2$/,
  en: /\/(unbounded-latin-600|manrope-latin-wght)-normal-[\w-]+\.woff2$/,
}

/** `/` and `/en` are the landing page; everything else is the app. */
function localeOf(url: string | undefined): Locale | undefined {
  const pathname = new URL(url ?? '/', 'http://x').pathname.replace(/\/$/, '') || '/'
  if (pathname === '/' || pathname === '/home') return 'uk'
  if (pathname === '/en') return 'en'
  return undefined
}

function sendHtml(res: ServerResponse, html: string) {
  res.statusCode = 200
  res.setHeader('Content-Type', 'text/html; charset=utf-8')
  res.end(html)
}

/**
 * Builds the landing page: plain HTML in two languages from landing/page.ts,
 * with the hashed styles and script of `landing/main.ts`. The app keeps
 * `index.html`; development and `vite preview` serve the landing at `/` and
 * `/en`, as Cloudflare does.
 */
export function landing(): Plugin {
  let outDir = ''
  return {
    name: 'landing',
    enforce: 'post',
    config: (config) => ({
      build: {
        rolldownOptions: {
          input: {
            index: path.resolve(config.root ?? '', 'index.html'),
            landing: path.resolve(config.root ?? '', ENTRY),
          },
        },
      },
    }),
    configResolved(config) {
      outDir = path.resolve(config.root, config.build.outDir)
    },

    configureServer(server) {
      server.middlewares.use((req: IncomingMessage, res: ServerResponse, next: () => void) => {
        const locale = req.method === 'GET' ? localeOf(req.url) : undefined
        if (!locale) return next()
        const html = renderPage(locale, {
          head: '',
          scripts: `<script type="module" src="/${ENTRY}"></script>`,
        })
        server
          .transformIndexHtml(req.url ?? '/', html)
          .then((page) => sendHtml(res, page))
          .catch(next)
      })
    },

    configurePreviewServer(server) {
      server.middlewares.use((req: IncomingMessage, res: ServerResponse, next: () => void) => {
        const locale = req.method === 'GET' ? localeOf(req.url) : undefined
        if (!locale) return next()
        readFile(path.join(outDir, FILES[locale]), 'utf8')
          .then((page) => sendHtml(res, page))
          .catch(next)
      })
    },

    generateBundle(_, bundle) {
      const entry = Object.values(bundle).find(
        (file): file is Rolldown.OutputChunk => file.type === 'chunk' && file.isEntry && file.name === 'landing',
      )
      if (!entry) throw new Error(`landing: no chunk for ${ENTRY}`)
      const css = stylesOf(entry, bundle)

      for (const [locale, fileName] of Object.entries(FILES) as [Locale, string][]) {
        const fonts = Object.keys(bundle).filter((file) => FONTS[locale].test(`/${file}`))
        const head = [
          ...fonts.map(
            (file) => `<link rel="preload" href="/${file}" as="font" type="font/woff2" crossorigin />`,
          ),
          ...css.map((file) => `<link rel="stylesheet" href="/${file}" />`),
        ].join('\n')
        this.emitFile({
          type: 'asset',
          fileName,
          source: renderPage(locale, {
            head,
            scripts: `<script type="module" src="/${entry.fileName}"></script>`,
          }),
        })
      }
    },
  }
}

/** The CSS files of a chunk and of everything it imports. */
function stylesOf(
  chunk: Rolldown.OutputChunk,
  bundle: Rolldown.OutputBundle,
  seen = new Set<string>(),
): string[] {
  if (seen.has(chunk.fileName)) return []
  seen.add(chunk.fileName)
  const own = [...(chunk.viteMetadata?.importedCss ?? [])]
  const imported = chunk.imports.flatMap((name) => {
    const file = bundle[name]
    return file?.type === 'chunk' ? stylesOf(file, bundle, seen) : []
  })
  return [...new Set([...imported, ...own])]
}
