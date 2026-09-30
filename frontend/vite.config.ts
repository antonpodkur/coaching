import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    // Telegram only opens Mini Apps over HTTPS, so development goes through a
    // cloudflared quick tunnel (see README).
    allowedHosts: ['.trycloudflare.com'],
    // The phone reaches the backend through the same tunnel: /api/* → :8080/*.
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },
})
