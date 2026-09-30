import createClient, { type Middleware } from 'openapi-fetch'

import type { components, paths } from './schema'

/** Request and response bodies, generated from the backend (`pnpm gen:api`). */
export type Schemas = components['schemas']

let sessionToken: string | null = null

/** Sets the bearer token sent with every request; `null` signs out. */
export function setSessionToken(token: string | null) {
  sessionToken = token
}

const auth: Middleware = {
  onRequest({ request }) {
    if (sessionToken) request.headers.set('Authorization', `Bearer ${sessionToken}`)
    return request
  },
}

export const api = createClient<paths>({ baseUrl: import.meta.env.VITE_API_URL })
api.use(auth)

/** A failed request, with the backend's machine-readable code (`not_a_client`, …). */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string) {
    super(`${status} ${code}`)
    this.status = status
    this.code = code
  }
}

/** Returns the body of a successful response and throws `ApiError` otherwise. */
export function unwrap<T>(result: { data?: T; error?: unknown; response: Response }): T {
  if (result.response.ok) return result.data as T
  const error = result.error
  const code =
    typeof error === 'object' && error !== null && 'error' in error
      ? String(error.error)
      : 'unknown'
  throw new ApiError(result.response.status, code)
}
