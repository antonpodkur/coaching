/**
 * The demo: the landing page runs the real app in two frames, the coach's and
 * a client's, against a made-up backend in the browser (src/demo/backend.ts).
 * A frame knows it is one by its name, `coachin-demo:<role>:<id>`; the id
 * links the two frames of one landing page to the same made-up data.
 */

export type DemoRole = 'coach' | 'client'

const match = /^coachin-demo:(coach|client):([\w-]{1,64})$/.exec(window.name)

/** This frame's part in the demo, or `null` for the real app. */
export const demo: { role: DemoRole; id: string } | null = match
  ? { role: match[1] as DemoRole, id: match[2] ?? 'demo' }
  : null

export function isDemo(): boolean {
  return demo !== null
}
