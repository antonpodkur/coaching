import { type QueryClient, useQuery } from '@tanstack/react-query'

import { type Schemas, api, unwrap } from '../api/client'
import { jpegBody, shrinkPhoto } from '../shared/photos'
import { SESSION_KEY } from './session'
import type { VideoTarget } from './videoSends'

export const QUESTIONNAIRE_KEY = ['my-questionnaire']
export const MAX_PHOTOS = 10
export const MAX_VIDEOS = 3
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

/** Shrinks a photo of the client's gym and stores it. */
export async function uploadPhoto(file: File): Promise<Schemas['GymPhoto']> {
  const jpeg = await shrinkPhoto(file)
  return unwrap(await api.POST('/me/gym/photos', jpegBody(jpeg)))
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
