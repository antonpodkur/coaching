import { BackLink } from '../shared/BackLink'
import { NutritionCard } from '../shared/NutritionCard'
import { Screen } from '../shared/Screen'
import { useCoachCard } from './coachCard'
import { useMyNutrition } from './nutrition'

/** The daily nutrition target Dasha set for the client. */
export function NutritionPage() {
  const nutrition = useMyNutrition()
  const coach = useCoachCard()
  return (
    <Screen>
      <BackLink to="/app" label="Головна" />
      <header className="exercise-head">
        <h1>Харчування</h1>
        <p className="muted small">Норма на кожен день, доки Даша її не змінить.</p>
      </header>
      {nutrition.isPending && <p className="muted">Завантаження…</p>}
      {nutrition.isError && <p className="error">Не вдалося завантажити.</p>}
      {nutrition.data && !nutrition.data.current && (
        <p className="muted">Даша ще не задала норму харчування. Коли задасть, вона з’явиться тут.</p>
      )}
      {nutrition.data?.current && <NutritionCard target={nutrition.data.current} coach={coach} />}
    </Screen>
  )
}
