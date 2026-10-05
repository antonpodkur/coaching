import type { QueryClient } from '@tanstack/react-query'

import { type Schemas, api, unwrap } from '../api/client'
import { jpegBody, squarePhoto } from '../shared/photos'
import { type AppSession, SESSION_KEY } from './session'

export type Avatar = Schemas['Avatar']

/** Cuts the photo to a square, shrinks it, and makes it the client's avatar. */
export async function uploadAvatar(file: File): Promise<Avatar> {
  const jpeg = await squarePhoto(file)
  return unwrap(await api.PUT('/me/avatar', jpegBody(jpeg)))
}

/** Removes the client's own photo; their Telegram photo may come back later. */
export async function removeAvatar() {
  unwrap(await api.DELETE('/me/avatar'))
}

/** Shows the new avatar everywhere at once: home, profile. */
export function showAvatar(queryClient: QueryClient, avatar: Avatar | null) {
  queryClient.setQueryData<AppSession>(SESSION_KEY, (session) =>
    session?.role === 'client' ? { ...session, client: { ...session.client, avatar } } : session,
  )
}
