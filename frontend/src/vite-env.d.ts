/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Backend base URL: `/api` in development (Vite proxy), the full URL in production. */
  readonly VITE_API_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
