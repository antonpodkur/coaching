/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Backend base URL: `/api` in development (Vite proxy), the full URL in production. */
  readonly VITE_API_URL: string
  /** Bot username without `@`, for the coach's Telegram Login Widget. */
  readonly VITE_BOT_USERNAME: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
