import { type QueryClient, useQuery } from '@tanstack/react-query'

import { type Schemas, api, unwrap } from '../api/client'
import { SESSION_KEY } from './session'
import type { VideoTarget } from './videoSends'

export const QUESTIONNAIRE_KEY = ['my-questionnaire']
export const MAX_PHOTOS = 10
export const MAX_VIDEOS = 3
/** Photos are shrunk to this on the longer side: plenty to see a gym, a few hundred KB. */
const PHOTO_SIDE = 1600
const PHOTO_QUALITY = 0.85

/** The client's own questionnaire. */
export function useQuestionnaire() {
  return useQuery({
    queryKey: QUESTIONNAIRE_KEY,
    queryFn: async () => unwrap(await api.GET('/me/questionnaire')),
  })
}

/** After the first answer or photo the home screen stops offering the questionnaire. */
export function markStarted(queryClient: QueryClient) {
  queryClient.setQueryData<Schemas['MiniAppSession']>(SESSION_KEY, (session) =>
    session?.role === 'client'
      ? { ...session, client: { ...session.client, questionnaire_started: true } }
      : session,
  )
}

/**
 * A photo as a JPEG at most `PHOTO_SIDE` px on the longer side. Redrawing it
 * also drops what the camera stored with it, such as the location.
 */
async function shrinkPhoto(file: File): Promise<Blob> {
  const url = URL.createObjectURL(file)
  try {
    const image = new Image()
    image.src = url
    await image.decode()
    const scale = Math.min(1, PHOTO_SIDE / Math.max(image.naturalWidth, image.naturalHeight))
    const canvas = document.createElement('canvas')
    canvas.width = Math.round(image.naturalWidth * scale)
    canvas.height = Math.round(image.naturalHeight * scale)
    const context = canvas.getContext('2d')
    if (!context) throw new Error('no canvas')
    context.drawImage(image, 0, 0, canvas.width, canvas.height)
    return await new Promise((resolve, reject) =>
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error('could not encode the photo'))),
        'image/jpeg',
        PHOTO_QUALITY,
      ),
    )
  } finally {
    URL.revokeObjectURL(url)
  }
}

/** Shrinks a photo of the client's gym and stores it. */
export async function uploadPhoto(file: File): Promise<Schemas['GymPhoto']> {
  const jpeg = await shrinkPhoto(file)
  return unwrap(
    await api.POST('/me/gym/photos', {
      // The body is the file itself, not JSON.
      body: jpeg as unknown as string,
      bodySerializer: (body) => body,
      headers: { 'Content-Type': 'image/jpeg' },
    }),
  )
}

/** Videos of the client's gym, in their questionnaire. */
export const GYM_VIDEOS: VideoTarget = {
  key: 'gym',
  start: async () => unwrap(await api.POST('/me/gym/videos')),
  uploaded: async (id) =>
    unwrap(await api.POST('/me/gym/videos/{id}/uploaded', { params: { path: { id } } })),
  remove: (id) => api.DELETE('/me/gym/{id}', { params: { path: { id } } }),
  refresh: (queryClient) => queryClient.invalidateQueries({ queryKey: QUESTIONNAIRE_KEY }),
}

const CARD_DISMISSED_KEY = 'questionnaire_card_dismissed'

/** "Не зараз" on the home screen's questionnaire card, on this phone. */
export function questionnaireCardDismissed(): boolean {
  try {
    return localStorage.getItem(CARD_DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

export function dismissQuestionnaireCard() {
  try {
    localStorage.setItem(CARD_DISMISSED_KEY, '1')
  } catch {
    // Without storage it simply shows again next time.
  }
}
