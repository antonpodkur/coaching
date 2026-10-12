import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'

import type { AppSession } from '../app/session'
import { SESSION_KEY } from '../app/session'
import { ClientApp } from '../app/ClientApp'
import { CoachWorkspace } from '../coach/CoachWorkspace'
import { clientProfile, coachProfile } from './backend'
import type { DemoRole } from './mode'
import { demoState, onDemoChange } from './transport'

/** Queries the other frame never changes under the screen: the builder's open draft. */
const KEEP = new Set(['coach-workout-builder', 'session'])

const noop = () => undefined

/**
 * The app as the landing page shows it: already signed in as the demo's coach
 * or client, with the made-up data. What one frame changes, the other shows.
 */
export function DemoApp({ role }: { role: DemoRole }) {
  const queryClient = useQueryClient()
  const [session] = useState((): AppSession => {
    const state = demoState()
    return role === 'coach'
      ? { role: 'coach', token: 'demo', coach: coachProfile(state) }
      : { role: 'client', token: 'demo', client: clientProfile(state) }
  })

  useEffect(() => {
    queryClient.setQueryData(SESSION_KEY, session)
  }, [queryClient, session])

  useEffect(
    () =>
      onDemoChange(() => {
        void queryClient.invalidateQueries({
          predicate: (query) => !KEEP.has(String(query.queryKey[0])),
        })
      }),
    [queryClient],
  )

  if (session.role === 'coach') {
    return <CoachWorkspace base="/app" coach={session.coach} onUnauthorized={noop} />
  }
  return <ClientApp client={session.client} />
}
