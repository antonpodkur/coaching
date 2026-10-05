import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'

import { type Schemas, api, unwrap } from '../api/client'
import { daysBetween, formatAge, formatDate, localDate } from '../shared/format'
import { CLIENTS_KEY, isUnauthorized, useCoach } from './context'

export type Client = Schemas['CoachClient']

export const SEX_WORD: Record<Schemas['Sex'], string> = { female: 'жіноча', male: 'чоловіча' }

/** One client's questionnaire: `[...CLIENTS_KEY, id, 'questionnaire']`. */
export function questionnaireQuery(id: string) {
  return queryOptions({
    queryKey: [...CLIENTS_KEY, id, 'questionnaire'],
    queryFn: async () =>
      unwrap(
        await api.GET('/coach/clients/{id}/questionnaire', { params: { path: { id } } }),
      ),
  })
}

/** `34 роки · чоловіча · 182 см · 6 фото · 1 відео`, or `null` before any answer. */
export function questionnaireSummary(client: Client): string | null {
  const parts = [
    client.birth_year != null && formatAge(client.birth_year),
    client.sex && SEX_WORD[client.sex],
    client.height_cm != null && `${client.height_cm} см`,
    client.gym_photos > 0 && `${client.gym_photos} фото`,
    client.gym_videos > 0 && `${client.gym_videos} відео`,
  ].filter(Boolean)
  return parts.length > 0 ? parts.join(' · ') : null
}

/** The archive list, fetched only when Dasha opens it. */
export const ARCHIVED_KEY = [...CLIENTS_KEY, 'archived']

/** Dasha's evening summary lists payments ending within this many days. */
const RENEWAL_DAYS = 3

export function clientQuery(id: string) {
  return queryOptions({
    queryKey: [...CLIENTS_KEY, id],
    queryFn: async () =>
      unwrap(await api.GET('/coach/clients/{id}', { params: { path: { id } } })),
  })
}

export interface Payment {
  /** Lower case, to follow other words: `оплачено до 8 жовтня`. */
  text: string
  /** Over, or ending soon enough that the bot reminds Dasha too. */
  due: boolean
}

export function payment(paidUntil: string | null | undefined): Payment | null {
  if (!paidUntil) return null
  const left = daysBetween(localDate(new Date()), paidUntil)
  if (left < 0) return { text: `оплата закінчилась ${formatDate(paidUntil)}`, due: true }
  return { text: `оплачено до ${formatDate(paidUntil)}`, due: left <= RENEWAL_DAYS }
}

/** Saves changes to one client and refreshes every list that shows them. */
export function useUpdateClient(id: string, onSaved?: (client: Client) => void) {
  const { onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (changes: Schemas['ClientChanges']) =>
      unwrap(
        await api.PATCH('/coach/clients/{id}', { params: { path: { id } }, body: changes }),
      ),
    onSuccess: (client) => {
      queryClient.setQueryData(clientQuery(id).queryKey, client)
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
      onSaved?.(client)
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })
}
