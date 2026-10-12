import { Suspense, lazy } from 'react'

import type { DemoRole } from './mode'

// Loaded only inside the landing page's demo frames, never by the real app.
const DemoApp = lazy(() => import('./DemoApp').then((module) => ({ default: module.DemoApp })))

export function LazyDemoApp({ role }: { role: DemoRole }) {
  return (
    <Suspense>
      <DemoApp role={role} />
    </Suspense>
  )
}
