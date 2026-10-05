import { useQuery } from '@tanstack/react-query'

import { api, unwrap } from '../api/client'

export const MY_NUTRITION_KEY = ['my-nutrition']

/** The client's daily target from Dasha; `current` is `null` until she sets one. */
export function useMyNutrition() {
  return useQuery({
    queryKey: MY_NUTRITION_KEY,
    queryFn: async () => unwrap(await api.GET('/me/nutrition')),
  })
}
