import { useQuery } from '@tanstack/react-query'

import { api, unwrap } from '../api/client'

export const MY_WEIGHT_KEY = ['my-weight']

/** The client's weigh-ins, oldest first. */
export function useMyWeight() {
  return useQuery({
    queryKey: MY_WEIGHT_KEY,
    queryFn: async () => unwrap(await api.GET('/me/weight')),
  })
}
