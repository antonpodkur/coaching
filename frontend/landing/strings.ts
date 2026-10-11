/**
 * Everything the landing page says, in Ukrainian (at /) and English (at /en).
 * The page is ours, so strings may carry a little markup (`<em>`, `&nbsp;`).
 * The phone pictures speak the page's language too, though the app itself is
 * in Ukrainian for now (the FAQ says so).
 */

/** Where a coach who is interested writes. */
export const CONTACT_URL = 'https://t.me/USERNAME'

export const uk = {
  lang: 'uk',
  ogLocale: 'uk_UA',
  title: 'Coachin — застосунок для онлайн-тренера',
  description:
    'Програми тренувань з вашими відео техніки, клієнти в Telegram без реєстрації і звіт після кожного тренування. Безкоштовно, поки триває бета.',

  nav: { coach: 'Для тренера', clients: 'Для клієнтів', beta: 'Ціна', client: 'Я клієнт', write: 'Написати' },

  hero: {
    eyebrow: 'Для онлайн-тренерів',
    title: 'Плани тренувань&nbsp;— <em>з переписки</em> в&nbsp;застосунок',
    lede: 'Складайте тренування з власної бібліотеки вправ, надсилайте клієнтам в один дотик і отримуйте звіт після кожного заняття. Клієнти відкривають усе просто з Telegram — без завантажень і паролів.',
    cta: 'Написати в Telegram',
    how: 'Як це працює',
    beta: '<b>Безкоштовно</b>, поки триває бета',
    then: 'Той самий план у Coachin',
    stageLabel: 'План тренування в переписці й той самий план у застосунку',
  },

  chat: {
    client: 'Андрій К.',
    initials: 'АК',
    online: 'в мережі',
    day: 'Понеділок, спина',
    exercises: [
      { name: 'Тяга верхнього блоку', sets: ['40 кг х 12', '45 х 10', '50 х 8'] },
      { name: 'Тяга гантелі в нахилі (на кожну руку)', sets: ['20 х 12', '22 х 10'] },
      { name: 'Підтягування', sets: ['х 8', 'х 8', 'х 6'] },
    ],
    sentAt: '09:12',
    replies: [
      { text: 'А можна ще раз відео тяги? Не можу знайти в чаті', at: '18:47' },
      { text: 'І скільки я минулого разу тягнув?', at: '18:48' },
    ],
  },

  builder: {
    back: 'Андрій К.',
    publish: 'Опублікувати',
    title: 'Спина',
    meta: 'Пн, 13 жовтня · 3 вправи',
    exercises: [
      { name: 'Тяга верхнього блоку', sub: 'Відео техніки', sets: ['40 × 12', '45 × 10', '50 × 8'] },
      { name: 'Тяга гантелі в нахилі', sub: 'на кожну руку', sets: ['20 × 12', '22 × 10'] },
      { name: 'Підтягування', sub: 'власна вага', sets: ['× 8', '× 8', '× 6'] },
    ],
    add: '+ Вправа з бібліотеки',
  },

  coach: {
    label: 'Для тренера',
    title: 'Від плану до звіту&nbsp;— без переписки',
    steps: [
      {
        title: 'Запросіть клієнта',
        text: 'Надішліть запрошення в Telegram. Клієнт відкриває застосунок одним дотиком — без реєстрації й паролів.',
      },
      {
        title: 'Складіть тренування',
        text: 'Вправи — з вашої бібліотеки, з вашими відео техніки. Кожен підхід окремо: розминка, робочі, дропсет. Наступний тиждень — копією попереднього.',
      },
      {
        title: 'Опублікуйте',
        text: 'Клієнт одразу отримує сповіщення, а в день тренування — нагадування. Писати «не забудь» більше не треба.',
      },
      {
        title: 'Перегляньте звіт',
        text: 'Після тренування — звіт: самопочуття, коментар і підходи, що відрізнялися від плану, першими. Відео техніки від клієнта — там само.',
      },
    ],
  },

  invite: { from: 'Coachin', text: 'Вас запрошено тренуватися в Coachin. Тут будуть ваші тренування, відео вправ і звіти.', open: 'Відкрити' },

  editor: {
    exercise: 'Тяга верхнього блоку',
    sets: ['40 × 12', '45 × 10', '50 × 8'],
    kg: 'кг',
    reps: 'повт.',
    copy: 'Скопіювати в наступне тренування',
  },

  notices: [
    { at: '09:12', title: 'Нове тренування: Спина', body: 'Пн, 13 жовтня · 3 вправи' },
    { at: '09:00', title: 'Сьогодні тренування', body: 'Спина · відкрити план' },
  ],

  report: {
    tag: 'Новий звіт · сьогодні, 19:40',
    title: 'Спина · 13 жовт',
    meta: '52 хв · 8 з 8 підходів · 1 інакше, ніж у плані',
    effort: 'Нормально',
    comment: '«Останній підхід тяги не дотягнув — передпліччя забилися»',
    section: 'Інакше, ніж у плані',
    exercise: 'Тяга гантелі в нахилі',
    side: 'на кожну руку',
    done: '20 × 12',
    off: '22 × 7 · план 10',
  },

  clients: {
    label: 'Для клієнтів',
    title: 'Клієнтам усе зрозуміло з першого дотику',
    points: [
      { title: 'Без завантажень', text: 'Відкривається з Telegram. За бажанням — на головний екран, як звичайний застосунок.' },
      { title: 'Відео під кожною вправою', text: 'Техніку більше не треба шукати в чаті.' },
      { title: 'Минулого разу', text: 'Поруч із планом видно, скільки було минулого тижня.' },
      { title: 'Працює без інтернету', text: 'Підходи збережуться й надішлються, щойно з’явиться зв’язок.' },
      { title: 'Відео для перевірки техніки', text: 'Клієнт знімає підхід і надсилає вам просто під вправою.' },
    ],
  },

  exercise: {
    count: 'Вправа 1 з 3',
    name: 'Тяга верхнього блоку',
    sets: '3 підходи',
    last: 'Минулого разу:',
    lastSets: '40 × 12 · 45 × 9 · 50 × 7',
    head: ['№', 'План', 'Кг', 'Повт.'],
    rows: [
      { plan: '40 × 12', kg: '40', reps: '12', done: true },
      { plan: '45 × 10', kg: '45', reps: '10', done: true },
      { plan: '50 × 8', kg: '50', reps: '8', done: false },
    ],
    progress: 'Виконано 2 з 3.',
    all: 'Усі вправи',
    next: 'Наступна',
  },

  more: {
    label: 'А ще',
    items: [
      { title: 'Анкета клієнта', text: 'Цілі, обмеження й фото із залу — ще до першого тренування.' },
      { title: 'Вага', text: 'Клієнт записує вагу, ви бачите тренд на графіку.' },
      { title: 'Харчування', text: 'Денні норми, які ви задали, — у клієнта перед очима.' },
      { title: 'Вечірній підсумок', text: 'Хто не відкрив тренування й у кого закінчується оплата.' },
      { title: 'З телефона', text: 'Уся робота тренера — з телефона, у Telegram або як застосунок.' },
      { title: 'І з комп’ютера', text: 'Та сама робоча зона в браузері, вхід підтверджується в Telegram.' },
    ],
  },

  beta: {
    title: 'Безкоштовно, поки триває бета',
    text: 'Напишіть мені в Telegram — покажу, як усе влаштовано, і допоможу підключити перших клієнтів. Про ціну після бети теж домовимося в переписці.',
    cta: 'Написати в Telegram',
  },

  faq: {
    label: 'Питання',
    items: [
      {
        q: 'Клієнтам треба щось встановлювати?',
        a: 'Ні. Coachin відкривається прямо з Telegram. Хто хоче — додає його на головний екран телефона, як звичайний застосунок.',
      },
      { q: 'Скільки це коштує?', a: 'Поки триває бета — безкоштовно. Про ціну після бети напишіть мені — обговоримо.' },
      {
        q: 'Хто бачить мої вправи й відео?',
        a: 'Вашу бібліотеку бачать лише ваші клієнти в застосунку. Відео, які надсилають клієнти, зберігаються приватно й відкриваються тільки вам.',
      },
      { q: 'Якою мовою застосунок?', a: 'Українською. Англійська — наступна.' },
    ],
  },

  footer: { client: 'Я клієнт — відкрити застосунок', install: 'Встановити на телефон' },
}

export type Strings = typeof uk

export const en: Strings = {
  lang: 'en',
  ogLocale: 'en_US',
  title: 'Coachin — the app for online coaches',
  description:
    'Training plans with your own technique videos, clients in Telegram with no sign-up, and a report after every workout. Free while in beta.',

  nav: { coach: 'For coaches', clients: 'For clients', beta: 'Pricing', client: 'I’m a client', write: 'Get in touch' },

  hero: {
    eyebrow: 'For online coaches',
    title: 'Training plans&nbsp;— <em>out of the chat</em>, into an&nbsp;app',
    lede: 'Build workouts from your own exercise library, send them to clients in one tap and get a report after every session. Clients open everything right from Telegram — no downloads, no passwords.',
    cta: 'Message me on Telegram',
    how: 'How it works',
    beta: '<b>Free</b> while in beta',
    then: 'The same plan in Coachin',
    stageLabel: 'A workout plan sent as a chat message, and the same plan in the app',
  },

  chat: {
    client: 'Andrew K.',
    initials: 'AK',
    online: 'online',
    day: 'Monday, back',
    exercises: [
      { name: 'Lat pulldown', sets: ['40 kg x 12', '45 x 10', '50 x 8'] },
      { name: 'One-arm dumbbell row (each arm)', sets: ['20 x 12', '22 x 10'] },
      { name: 'Pull-ups', sets: ['x 8', 'x 8', 'x 6'] },
    ],
    sentAt: '09:12',
    replies: [
      { text: 'Could you send the pulldown video again? Can’t find it in the chat', at: '18:47' },
      { text: 'And how much did I lift last time?', at: '18:48' },
    ],
  },

  builder: {
    back: 'Andrew K.',
    publish: 'Publish',
    title: 'Back',
    meta: 'Mon, 13 October · 3 exercises',
    exercises: [
      { name: 'Lat pulldown', sub: 'Technique video', sets: ['40 × 12', '45 × 10', '50 × 8'] },
      { name: 'One-arm dumbbell row', sub: 'each arm', sets: ['20 × 12', '22 × 10'] },
      { name: 'Pull-ups', sub: 'bodyweight', sets: ['× 8', '× 8', '× 6'] },
    ],
    add: '+ Exercise from library',
  },

  coach: {
    label: 'For coaches',
    title: 'From plan to report&nbsp;— no chat threads',
    steps: [
      {
        title: 'Invite a client',
        text: 'Send an invite in Telegram. Your client opens the app in one tap — no sign-up, no passwords.',
      },
      {
        title: 'Build the workout',
        text: 'Exercises come from your library, with your technique videos. Every set on its own: warm-up, working sets, drop set. Next week starts as a copy of this one.',
      },
      {
        title: 'Publish',
        text: 'Your client gets a notification right away and a reminder on workout day. No more “don’t forget” messages.',
      },
      {
        title: 'Read the report',
        text: 'After the workout: how it felt, a comment, and the sets that differed from the plan, first. Your client’s form videos are right there too.',
      },
    ],
  },

  invite: { from: 'Coachin', text: 'You’re invited to train in Coachin. Your workouts, exercise videos and reports will be here.', open: 'Open' },

  editor: {
    exercise: 'Lat pulldown',
    sets: ['40 × 12', '45 × 10', '50 × 8'],
    kg: 'kg',
    reps: 'reps',
    copy: 'Copy to the next workout',
  },

  notices: [
    { at: '09:12', title: 'New workout: Back', body: 'Mon, 13 October · 3 exercises' },
    { at: '09:00', title: 'Workout today', body: 'Back · open the plan' },
  ],

  report: {
    tag: 'New report · today, 19:40',
    title: 'Back · 13 Oct',
    meta: '52 min · 8 of 8 sets · 1 differs from the plan',
    effort: 'OK',
    comment: '“Couldn’t finish the last row set — forearms gave out”',
    section: 'Different from the plan',
    exercise: 'One-arm dumbbell row',
    side: 'each arm',
    done: '20 × 12',
    off: '22 × 7 · plan 10',
  },

  clients: {
    label: 'For clients',
    title: 'Clear to clients from the first tap',
    points: [
      { title: 'Nothing to download', text: 'Opens from Telegram. Add it to the home screen if you like, like any other app.' },
      { title: 'A video under every exercise', text: 'No more scrolling the chat for technique.' },
      { title: 'Last time', text: 'Next to the plan: what they did last week.' },
      { title: 'Works offline', text: 'Sets are saved and sent as soon as there’s signal.' },
      { title: 'Form checks', text: 'Your client films a set and sends it to you right under the exercise.' },
    ],
  },

  exercise: {
    count: 'Exercise 1 of 3',
    name: 'Lat pulldown',
    sets: '3 sets',
    last: 'Last time:',
    lastSets: '40 × 12 · 45 × 9 · 50 × 7',
    head: ['#', 'Plan', 'Kg', 'Reps'],
    rows: [
      { plan: '40 × 12', kg: '40', reps: '12', done: true },
      { plan: '45 × 10', kg: '45', reps: '10', done: true },
      { plan: '50 × 8', kg: '50', reps: '8', done: false },
    ],
    progress: '2 of 3 done.',
    all: 'All exercises',
    next: 'Next',
  },

  more: {
    label: 'And also',
    items: [
      { title: 'Client questionnaire', text: 'Goals, limitations and gym photos before the first workout.' },
      { title: 'Weight', text: 'Your client logs their weight; you see the trend on a chart.' },
      { title: 'Nutrition', text: 'The daily targets you set, always in front of your client.' },
      { title: 'Evening summary', text: 'Who hasn’t opened their workout, and whose payment is ending.' },
      { title: 'From your phone', text: 'All of a coach’s work from the phone, in Telegram or as an app.' },
      { title: 'And your computer', text: 'The same workspace in a browser; you confirm sign-in in Telegram.' },
    ],
  },

  beta: {
    title: 'Free while in beta',
    text: 'Message me on Telegram — I’ll show you around and help you bring your first clients over. We’ll agree on pricing after the beta there too.',
    cta: 'Message me on Telegram',
  },

  faq: {
    label: 'Questions',
    items: [
      {
        q: 'Do clients need to install anything?',
        a: 'No. Coachin opens right from Telegram. Anyone who wants can add it to their phone’s home screen like a regular app.',
      },
      { q: 'How much does it cost?', a: 'It’s free while in beta. For pricing after the beta, message me and we’ll talk.' },
      {
        q: 'Who sees my exercises and videos?',
        a: 'Your library is shown only to your clients in the app. Videos your clients send are stored privately and open only for you.',
      },
      { q: 'What language is the app in?', a: 'Ukrainian for now. English is next.' },
    ],
  },

  footer: { client: 'I’m a client — open the app', install: 'Install on your phone' },
}
