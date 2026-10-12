import { type DemoState, handle } from './backend'
import { demo } from './mode'
import { createState } from './seed'

/**
 * Carries the app's requests to the demo backend instead of the network.
 *
 * Both frames of one landing page share one copy of the data: it is kept as
 * a JSON string on the landing page's window, under the frames' common id,
 * so either frame can read it and nothing outlives the page. After a change
 * the frame says so on a BroadcastChannel, and the other frame refreshes.
 */

const HOLDER = '__coachinDemo'
const DELAY_MS = 140

type Holder = Record<string, string>

/** The landing page's window, or this one when the demo runs on its own. */
function holder(): Holder {
  const host = (() => {
    try {
      // Reading a property throws if the page around the frame is not ours.
      void (window.parent as unknown as Record<string, unknown>)[HOLDER]
      return window.parent as unknown as Record<string, unknown>
    } catch {
      return window as unknown as Record<string, unknown>
    }
  })()
  host[HOLDER] ??= {}
  return host[HOLDER] as Holder
}

const key = demo?.id ?? 'demo'
const channel = 'BroadcastChannel' in window ? new BroadcastChannel(`coachin-demo:${key}`) : null

function load(): DemoState {
  const saved = holder()[key]
  if (saved) return JSON.parse(saved) as DemoState
  const state = createState()
  holder()[key] = JSON.stringify(state)
  return state
}

function save(state: DemoState) {
  holder()[key] = JSON.stringify(state)
  channel?.postMessage('changed')
}

/** The demo's data as it is now, for the app's first screen. */
export function demoState(): DemoState {
  return load()
}

/** Calls `listener` whenever the other frame changes the data. */
export function onDemoChange(listener: () => void): () => void {
  const receive = () => listener()
  channel?.addEventListener('message', receive)
  return () => channel?.removeEventListener('message', receive)
}

const apiBase = new URL(import.meta.env.VITE_API_URL, window.location.href).href.replace(/\/$/, '')

/** `fetch` for the API client in the demo: answers from the demo backend. */
export async function demoFetch(request: Request): Promise<Response> {
  const url = new URL(request.url)
  const address = url.origin + url.pathname
  const path = address.startsWith(apiBase) ? address.slice(apiBase.length) : url.pathname
  const raw = request.method === 'GET' || request.method === 'HEAD' ? '' : await request.text()
  let body: unknown
  try {
    body = raw ? JSON.parse(raw) : undefined
  } catch {
    body = undefined
  }
  // A real request takes a moment; the app's loading states should too.
  await new Promise((resolve) => setTimeout(resolve, DELAY_MS))

  const state = load()
  const { response, changed } = handle(state, {
    method: request.method,
    path,
    query: Object.fromEntries(url.searchParams),
    body,
    ifMatch: request.headers.get('If-Match'),
    role: demo?.role ?? 'client',
  })
  if (changed) save(state)
  if (response.body === undefined) return new Response(null, { status: response.status })
  return new Response(JSON.stringify(response.body), {
    status: response.status,
    headers: { 'Content-Type': 'application/json' },
  })
}
