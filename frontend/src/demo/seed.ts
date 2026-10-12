import {
  type DemoClient,
  type DemoExercise,
  type DemoReport,
  type DemoRow,
  type DemoState,
  type DemoWorkout,
  localDay,
  localTime,
} from './backend'

/**
 * The demo's made-up coach, clients and history, dated around today. Андрій
 * is the client whose phone the demo shows: today's "Спина" waits as a draft
 * for the coach to publish, and last week's has what he did, for "Минулого
 * разу". Марта has a report the coach hasn't read; Ігор's payment is ending;
 * Ліза hasn't opened her invite yet.
 */

const id = () => crypto.randomUUID()

const LIBRARY: [name: string, measure: DemoExercise['measure'], group: string, description?: string][] = [
  ['Тяга верхнього блоку', 'weight', 'Спина', 'Хват трохи ширший за плечі. Тягни до верхньої частини грудей, лопатки вниз.'],
  ['Тяга гантелі в нахилі', 'weight', 'Спина', 'Упор коліном і рукою в лаву, спина рівна. Тягни лікоть до пояса.'],
  ['Підтягування', 'bodyweight', 'Спина'],
  ['Тяга горизонтального блоку', 'weight', 'Спина'],
  ['Гіперекстензія', 'bodyweight', 'Спина'],
  ['Присідання зі штангою', 'weight', 'Ноги', 'Стопи на ширині плечей, коліна йдуть за носками. Глибина — до паралелі.'],
  ['Жим ногами', 'weight', 'Ноги'],
  ['Розгинання ніг у тренажері', 'weight', 'Ноги'],
  ['Випади з гантелями', 'weight', 'Ноги'],
  ['Румунська тяга', 'weight', 'Сідниці', 'Коліна злегка зігнуті, гриф ковзає по стегнах. Спина рівна весь рух.'],
  ['Ягідний місток зі штангою', 'weight', 'Сідниці'],
  ['Жим штанги лежачи', 'weight', 'Груди'],
  ['Жим гантелей на похилій лаві', 'weight', 'Груди'],
  ['Віджимання на брусах', 'bodyweight', 'Груди'],
  ['Жим гантелей сидячи', 'weight', 'Плечі'],
  ['Махи гантелями в сторони', 'weight', 'Плечі'],
  ['Згинання рук з гантелями', 'weight', 'Руки'],
  ['Розгинання рук на блоці', 'weight', 'Руки'],
  ['Планка', 'time', 'Прес'],
  ['Скручування', 'bodyweight', 'Прес'],
]

/** A planned set: kg (`null` without weight) and reps, or a rep range. */
type Target = [kg: number | null, reps: number, repsMax?: number]
/** What the client did instead of the plan; `null` is a set left out. */
type Did = [kg: number | null, reps: number] | null

interface RowSpec {
  exercise: string
  sets: Target[]
  /** By set: what was done differently. Sets not listed went as planned. */
  did?: Record<number, Did>
  side?: string
  note?: string
}

interface WorkoutSpec {
  title: string
  day: number
  status: DemoWorkout['status']
  rows: RowSpec[]
  report?: Omit<DemoReport, 'finished_at' | 'seen_at'> & { seen: boolean }
}

function client(
  name: string,
  joinedDaysAgo: number,
  details: Partial<DemoClient> = {},
): DemoClient {
  return {
    id: id(),
    name,
    created_at: localTime(-joinedDaysAgo, 12),
    paid_until: null,
    archived_at: null,
    joined: true,
    invite_expires_at: null,
    birth_year: null,
    height_cm: null,
    sex: null,
    weights: [],
    targets: [],
    ...details,
  }
}

export function createState(): DemoState {
  const exercises: DemoExercise[] = LIBRARY.map(([name, measure, group, description]) => ({
    id: id(),
    name,
    measure,
    muscle_group: group,
    description: description ?? null,
    in_library: true,
    archived: false,
  }))
  const exerciseId = (name: string) => {
    const found = exercises.find((exercise) => exercise.name === name)
    if (!found) throw new Error(`demo seed: no exercise ${name}`)
    return found.id
  }

  const workouts: DemoWorkout[] = []
  const add = (owner: DemoClient, spec: WorkoutSpec): string => {
    const done = spec.status === 'done'
    let minute = 0
    const rows: DemoRow[] = spec.rows.map((row) => ({
      id: id(),
      exercise_id: exerciseId(row.exercise),
      per_side_label: row.side ?? null,
      note: row.note ?? null,
      sets: row.sets.map(([kg, reps, repsMax], index) => {
        const did = row.did?.[index]
        const completed = done && did !== null
        const at = completed ? localTime(spec.day, 18, (minute += 3)) : null
        return {
          id: id(),
          target_kg: kg,
          target_reps_min: reps,
          target_reps_max: repsMax ?? reps,
          actual_kg: completed ? (did ? did[0] : kg) : null,
          actual_reps: completed ? (did ? did[1] : (repsMax ?? reps)) : null,
          completed_at: at,
          client_updated_at: at,
        }
      }),
    }))
    workouts.push({
      id: id(),
      client_id: owner.id,
      title: spec.title,
      date: localDay(spec.day),
      status: spec.status,
      version: 1,
      published_at: spec.status === 'draft' ? null : localTime(spec.day - 2, 20),
      opened_at: done ? localTime(spec.day, 18) : null,
      created_at: localTime(spec.day - 3, 21),
      rows,
      report: spec.report
        ? {
            effort: spec.report.effort,
            comment: spec.report.comment,
            duration_min: spec.report.duration_min,
            finished_at: localTime(spec.day, 19, 40),
            seen_at: spec.report.seen ? localTime(spec.day, 21) : null,
          }
        : null,
    })
    return workouts[workouts.length - 1]?.id ?? ''
  }

  const back = (did: { pulldown?: Record<number, Did>; pullups?: Record<number, Did> } = {}): RowSpec[] => [
    {
      exercise: 'Тяга верхнього блоку',
      sets: [[40, 12], [45, 10], [50, 8]],
      did: did.pulldown,
      note: 'Без ривків, лопатки вниз.',
    },
    { exercise: 'Тяга гантелі в нахилі', sets: [[20, 12], [22, 10]], side: 'на кожну руку' },
    { exercise: 'Підтягування', sets: [[null, 8], [null, 8], [null, 6]], did: did.pullups },
  ]

  const andrii = client('Андрій К.', 40, {
    paid_until: localDay(18),
    birth_year: 1994,
    height_cm: 182,
    sex: 'male',
    weights: [0, 3, 7, 10, 14, 17, 21, 24, 28].map((ago, index) => ({
      date: localDay(-ago),
      kg: Math.round((83.2 + index * 0.18 + (index % 2) * 0.15) * 10) / 10,
    })),
    targets: [
      {
        id: id(),
        protein_g: 160,
        fat_g: 70,
        carbs_g: 260,
        kcal: 4 * 160 + 4 * 260 + 9 * 70,
        note: '2 л води на день',
        set_at: localTime(-20, 10),
      },
    ],
  })
  add(andrii, {
    title: 'Спина',
    day: -7,
    status: 'done',
    rows: back({ pulldown: { 1: [45, 9], 2: [50, 7] }, pullups: { 2: [null, 5] } }),
    report: { effort: 'ok', comment: 'Підтягування в кінці пішли важко', duration_min: 54, seen: true },
  })
  add(andrii, {
    title: 'Ноги',
    day: -5,
    status: 'done',
    rows: [
      { exercise: 'Присідання зі штангою', sets: [[60, 10], [70, 8], [75, 6]] },
      { exercise: 'Жим ногами', sets: [[120, 12], [120, 12], [120, 12]] },
      { exercise: 'Розгинання ніг у тренажері', sets: [[35, 15], [35, 15]] },
    ],
    report: { effort: 'ok', comment: '', duration_min: 48, seen: true },
  })
  add(andrii, {
    title: 'Груди + плечі',
    day: -3,
    status: 'done',
    rows: [
      { exercise: 'Жим штанги лежачи', sets: [[60, 10], [65, 8], [70, 6]] },
      { exercise: 'Жим гантелей сидячи', sets: [[18, 10], [18, 10], [18, 10]] },
      { exercise: 'Махи гантелями в сторони', sets: [[8, 15], [8, 15], [8, 15]] },
    ],
    report: { effort: 'easy', comment: 'Можна додати вагу на жимі', duration_min: 50, seen: true },
  })
  // What the demo starts with: the coach publishes this, the client does it.
  const featured = add(andrii, { title: 'Спина', day: 0, status: 'draft', rows: back() })
  add(andrii, {
    title: 'Ноги',
    day: 2,
    status: 'published',
    rows: [
      { exercise: 'Присідання зі штангою', sets: [[60, 10], [70, 8], [77.5, 6]] },
      { exercise: 'Румунська тяга', sets: [[60, 10], [60, 10], [60, 10]] },
      { exercise: 'Випади з гантелями', sets: [[14, 10], [14, 10]], side: 'на кожну ногу' },
    ],
  })

  const marta = client('Марта Л.', 25, {
    paid_until: localDay(25),
    birth_year: 1998,
    height_cm: 168,
    sex: 'female',
  })
  add(marta, {
    title: 'Сідниці + ноги',
    day: -1,
    status: 'done',
    rows: [
      { exercise: 'Румунська тяга', sets: [[50, 10], [50, 10], [50, 10]], did: { 2: [50, 8] } },
      { exercise: 'Ягідний місток зі штангою', sets: [[70, 12], [70, 12], [70, 12]] },
      { exercise: 'Випади з гантелями', sets: [[10, 12], [10, 12]], side: 'на кожну ногу' },
    ],
    report: {
      effort: 'hard',
      comment: 'Румунська тяга важко пішла — в останньому підході 8 замість 10',
      duration_min: 61,
      seen: false,
    },
  })
  add(marta, {
    title: 'Спина + плечі',
    day: 2,
    status: 'published',
    rows: [
      { exercise: 'Тяга горизонтального блоку', sets: [[35, 12], [40, 10], [40, 10]] },
      { exercise: 'Махи гантелями в сторони', sets: [[5, 15], [5, 15], [5, 15]] },
    ],
  })

  const ihor = client('Ігор С.', 60, { paid_until: localDay(2), birth_year: 1989, sex: 'male' })
  add(ihor, {
    title: 'Груди',
    day: -4,
    status: 'done',
    rows: [
      { exercise: 'Жим штанги лежачи', sets: [[80, 8], [85, 6], [85, 6]] },
      { exercise: 'Віджимання на брусах', sets: [[null, 12], [null, 10]] },
    ],
    report: { effort: 'ok', comment: '', duration_min: 45, seen: true },
  })
  add(ihor, {
    title: 'Груди + трицепс',
    day: 1,
    status: 'published',
    rows: [
      { exercise: 'Жим гантелей на похилій лаві', sets: [[30, 10], [32, 8], [32, 8]] },
      { exercise: 'Розгинання рук на блоці', sets: [[25, 12], [25, 12], [25, 12]] },
    ],
  })

  const liza = client('Ліза Д.', 1, {
    joined: false,
    invite_expires_at: localTime(6, 12),
  })

  return {
    coach: {
      id: id(),
      name: 'Олена',
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Kyiv',
    },
    clientId: andrii.id,
    featured,
    clients: [andrii, marta, ihor, liza],
    exercises,
    workouts,
  }
}
