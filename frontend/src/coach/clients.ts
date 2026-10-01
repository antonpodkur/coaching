import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'

import { type Schemas, api, unwrap } from '../api/client'
import { daysBetween, formatDate, localDate } from '../shared/format'
import { CLIENTS_KEY, isUnauthorized, useCoach } from './context'

export type Client = Schemas['CoachClient']

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
