import { queryOptions } from '@tanstack/react-query'

import { api, unwrap } from '../api/client'
import { RESULTS_KEY } from './context'

/** A workout's results, for the report. */
export function resultsQuery(workoutId: string) {
  return queryOptions({
    queryKey: [...RESULTS_KEY, workoutId],
    queryFn: async () =>
      unwrap(
        await api.GET('/coach/workouts/{id}/results', { params: { path: { id: workoutId } } }),
      ),
  })
}
