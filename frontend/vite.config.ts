import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

import { serviceWorker } from './sw/plugin.ts'

// The phone reaches the backend through the same address: /api/* → :8080/*.
const apiProxy = {
  '/api': {
    target: 'http://localhost:8080',
    rewrite: (path: string) => path.replace(/^\/api/, ''),
  },
}

export default defineConfig({
  plugins: [react(), serviceWorker()],
  server: {
    port: 5173,
    // Telegram only opens Mini Apps over HTTPS, so development goes through a
    // cloudflared quick tunnel (see README).
    allowedHosts: ['.trycloudflare.com'],
    proxy: apiProxy,
  },
  // `pnpm build && pnpm preview` runs the built app with its service worker,
  // which development leaves out.
  preview: { proxy: apiProxy },
})
