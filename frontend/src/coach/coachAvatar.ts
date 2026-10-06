import type { QueryClient } from '@tanstack/react-query'

import { api, unwrap } from '../api/client'
import { type AppSession, SESSION_KEY } from '../app/session'
import { jpegBody, squarePhoto } from '../shared/photos'

/** Cuts the photo to a square, shrinks it, and makes it the coach's photo. */
export async function uploadCoachAvatar(file: File): Promise<string> {
  const jpeg = await squarePhoto(file)
  return unwrap(await api.PUT('/coach/me/avatar', jpegBody(jpeg))).url
}

export async function removeCoachAvatar() {
  unwrap(await api.DELETE('/coach/me/avatar'))
}

/** Shows the new photo everywhere at once: her profile, the person button. */
export function showCoachAvatar(queryClient: QueryClient, url: string | null) {
  queryClient.setQueryData<AppSession>(SESSION_KEY, (session) =>
    session?.role === 'coach' ? { ...session, coach: { ...session.coach, avatar_url: url } } : session,
  )
}
