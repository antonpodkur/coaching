import type { Schemas } from '../api/client'
import { formatToday } from '../shared/format'
import { Screen } from '../shared/Screen'

/** The client's start screen. Workouts arrive here in a later step. */
export function ClientHome({ client }: { client: Schemas['ClientProfile'] }) {
  const firstName = client.name.split(' ')[0]
  return (
    <Screen>
      <p className="muted">{formatToday()}</p>
      <h1>Привіт, {firstName}</h1>
      <p className="muted">Тут з’являться тренування від Даші.</p>
    </Screen>
  )
}
