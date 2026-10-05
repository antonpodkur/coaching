import { createHash } from 'node:crypto'
import { readFile, readdir, writeFile } from 'node:fs/promises'
import path from 'node:path'

import type { Plugin } from 'vite'

/** Only used when the app is installed, or not by the app at all. */
const NOT_KEPT = /^(_headers|sw\.js)$|\.(png|woff)$/
const PLACEHOLDER = "{ version: 'dev', files: [] }"

/**
 * Writes `dist/sw.js` from `sw/sw.js` after the build, with the list of files
 * the phone should keep and a version that changes whenever one of them does.
 * A changed `sw.js` is what tells phones that a new version is out.
 */
export function serviceWorker(): Plugin {
  let outDir = ''
  return {
    name: 'service-worker',
    apply: 'build',
    configResolved(config) {
      outDir = path.resolve(config.root, config.build.outDir)
    },
    async closeBundle() {
      const template = await readFile(new URL('./sw.js', import.meta.url), 'utf8')
      if (template.split(PLACEHOLDER).length !== 2) {
        throw new Error(`sw/sw.js must contain ${PLACEHOLDER} exactly once`)
      }

      const files = (await readdir(outDir, { recursive: true, withFileTypes: true }))
        .filter((entry) => entry.isFile())
        .map((entry) => path.relative(outDir, path.join(entry.parentPath, entry.name)))
        .map((file) => file.split(path.sep).join('/'))
        .filter((file) => !NOT_KEPT.test(file))
        .sort()

      const version = createHash('sha256').update(template)
      for (const file of files) {
        version.update(file).update(await readFile(path.join(outDir, file)))
      }
      const build = {
        version: version.digest('hex').slice(0, 12),
        files: files.map((file) => (file === 'index.html' ? '/' : `/${file}`)),
      }
      await writeFile(
        path.join(outDir, 'sw.js'),
        template.replace(PLACEHOLDER, JSON.stringify(build)),
      )
    },
  }
}
