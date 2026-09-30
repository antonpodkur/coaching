import type { ReactNode } from 'react'

/** Full-height phone screen with the Mini App's side gutters. */
export function Screen({ children }: { children: ReactNode }) {
  return <main className="screen">{children}</main>
}
