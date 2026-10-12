import { CONTACT_URL, type Strings, en, uk } from './strings.ts'

/**
 * The landing page as plain HTML, one copy per language. The build writes
 * them out (landing/plugin.ts); nothing here runs in the browser. The phone
 * pictures use the app's own classes and sizes (`.app`, landing.css), drawn
 * rather than screenshotted so they can speak either language.
 */

export type Locale = 'uk' | 'en'

export const ORIGIN = 'https://getcoachin.app'
export const PATHS: Record<Locale, string> = { uk: '/', en: '/en' }
const STRINGS: Record<Locale, Strings> = { uk, en }

export interface Assets {
  /** Stylesheets, font preloads and the like, for <head>. */
  head: string
  /** The page's script, at the end of <body>. */
  scripts: string
}

const plane =
  '<svg class="plane" viewBox="0 0 24 24" aria-hidden="true"><path d="M21.5 3.5L2.8 10.7c-.9.4-.9 1.6.1 1.9l4.6 1.5 1.8 5.6c.3.9 1.4 1.1 2 .4l2.6-2.7 4.7 3.5c.7.5 1.7.1 1.9-.8l3-14.6c.2-1-.8-1.8-1.8-1.5zM9.7 14.4l-.6 3.6-1.2-3.9 9.6-6.6-7.8 6.9z" fill="currentColor"/></svg>'

const contact = (label: string, extra = '') =>
  `<a class="btn ${extra}" href="${CONTACT_URL}" target="_blank" rel="noopener">${plane}${label}</a>`

/** A phone frame around an app screen drawn at the app's own 390×844. */
const phone = (screen: string, label: string) => `
<div class="phone" role="img" aria-label="${label}">
  <div class="island"></div>
  <div class="screen"><div class="app" aria-hidden="true">
    <div class="status"><span>9:41</span><span class="signal"><i></i><i></i><i></i><b></b></span></div>
    <div class="body">${screen}</div>
  </div></div>
</div>`

function hero(t: Strings): string {
  const chat = t.chat
  const lines = chat.exercises
    .map(
      (exercise, index) =>
        `<span class="l ex" data-g="${index}">${exercise.name}</span>` +
        exercise.sets.map((set) => `<span class="l" data-g="${index}">${set}</span>`).join(''),
    )
    .join('')
  const replies = chat.replies
    .map((reply) => `<div class="bubble in">${reply.text}<time>${reply.at}</time></div>`)
    .join('')
  const cards = t.builder.exercises
    .map(
      (exercise, index) => `
      <div class="card" data-g="${index}">
        <div class="ex-head"><span class="thumb"></span><div><div class="ex-name">${exercise.name}</div><div class="ex-sub">${exercise.sub}</div></div><span class="dots">···</span></div>
        <div class="chips">${exercise.sets.map((set, n) => `<span class="set-chip"><b>${n + 1}</b>${set}</span>`).join('')}<span class="set-chip add">+</span></div>
      </div>`,
    )
    .join('')

  return `
<section class="hero">
  <div class="hero-text">
    <span class="eyebrow">${t.hero.eyebrow}</span>
    <h1>${t.hero.title}</h1>
    <p class="lede">${t.hero.lede}</p>
    <div class="actions">
      ${contact(t.hero.cta, 'big')}
      <a class="quiet" href="#demo">${t.hero.how} ↓</a>
    </div>
    <p class="beta-note"><span>${t.hero.beta}</span></p>
  </div>

  <div class="stage" id="stage" role="img" aria-label="${t.hero.stageLabel}">
    <svg class="threads" id="threads" aria-hidden="true"></svg>
    <div class="chat" aria-hidden="true">
      <div class="chat-head"><span class="av">${chat.initials}</span><span>${chat.client}<small>${chat.online}</small></span></div>
      <div class="msgs">
        <div class="bubble out"><span class="l">${chat.day}</span>${lines}<time>${chat.sentAt} ✓✓</time></div>
        ${replies}
      </div>
    </div>
    <div class="then" aria-hidden="true">${t.hero.then}</div>
    ${phone(
      `<div class="topbar"><span class="back">‹ ${t.builder.back}</span><span class="btn-app small primary">${t.builder.publish}</span></div>
      <div><div class="h1">${t.builder.title}</div><div class="muted small gap">${t.builder.meta}</div></div>
      ${cards}
      <div class="dashed">${t.builder.add}</div>`,
      t.hero.then,
    )}
  </div>
</section>`
}

/**
 * The real app, twice: the coach's phone and the client's, each in a frame
 * running the app's demo mode (src/demo) against the same made-up data. The
 * frames load as the section comes near (landing/main.ts), and the steps
 * between them tick off as the visitor goes.
 */
function demo(t: Strings): string {
  const d = t.demo
  const device = (role: 'coach' | 'client', title: string) => `
    <div class="phone">
      <div class="island"></div>
      <div class="screen"><div class="app">
        <div class="status" aria-hidden="true"><span>9:41</span><span class="signal"><i></i><i></i><i></i><b></b></span></div>
        <div class="frame-wrap" data-role="${role}" data-title="${title}">
          <span class="frame-loading" aria-hidden="true">${d.loading}</span>
        </div>
      </div></div>
    </div>`
  const steps = d.steps
    .map(
      (step, index) =>
        `<li data-step="${index}"><span class="step-who ${step.who}">${step.who === 'coach' ? d.coach : d.client}</span><span class="step-say">${step.text}</span></li>`,
    )
    .join('')
  return `
<section class="section demo" id="demo" data-show="coach">
  <div class="section-head">
    <span class="label">${d.label}</span>
    <h2>${d.title}</h2>
    <p class="demo-lede">${d.lede}</p>
  </div>
  <div class="demo-switch" role="group" aria-label="${d.label}">
    <button type="button" data-show="coach" aria-pressed="true">${d.coach}</button>
    <button type="button" data-show="client" aria-pressed="false">${d.client}</button>
  </div>
  <div class="demo-stage">
    <figure class="demo-phone" data-role="coach">
      <figcaption><span class="step-who coach">${d.coach}</span>${d.coachName}</figcaption>
      ${device('coach', d.coachFrame)}
    </figure>
    <div class="demo-guide">
      <span class="label">${d.stepsLabel}</span>
      <ol class="demo-steps" aria-live="polite">${steps}</ol>
      <p class="demo-done">${d.done}</p>
      ${d.note ? `<p class="demo-note">${d.note}</p>` : ''}
      <button type="button" class="link-reset" data-reset>${d.reset}</button>
    </div>
    <figure class="demo-phone" data-role="client">
      <figcaption><span class="step-who client">${d.client}</span>${d.clientName}</figcaption>
      ${device('client', d.clientFrame)}
    </figure>
  </div>
</section>`
}

/** Small pieces of the app next to each step, at the app's real size. */
function fragments(t: Strings): string[] {
  const editor = t.editor
  const report = t.report
  return [
    `<div class="frag chat-frag">
      <div class="bubble in bot"><strong>${t.invite.from}</strong>${t.invite.text}<span class="inline-btn">${t.invite.open}</span></div>
    </div>`,
    `<div class="frag app ui">
      <div class="card">
        <div class="ex-head"><span class="thumb"></span><div><div class="ex-name">${editor.exercise}</div></div><span class="dots">···</span></div>
        <div class="chips">${editor.sets.map((set, n) => `<span class="set-chip${n === 1 ? ' on' : ''}"><b>${n + 1}</b>${set}</span>`).join('')}<span class="set-chip add">+</span></div>
      </div>
      <div class="steppers">
        <div class="stepper"><span class="step-btn">−</span><span class="step-value">45<small>${editor.kg}</small></span><span class="step-btn">+</span></div>
        <div class="stepper"><span class="step-btn">−</span><span class="step-value">10<small>${editor.reps}</small></span><span class="step-btn">+</span></div>
      </div>
      <span class="btn-app block">${editor.copy}</span>
    </div>`,
    `<div class="frag notices">
      ${t.notices
        .map(
          (notice) => `<div class="notice"><span class="notice-icon">C</span><span class="notice-meta"><span>Coachin</span><span>${notice.at}</span></span><strong>${notice.title}</strong><span>${notice.body}</span></div>`,
        )
        .join('')}
    </div>`,
    `<div class="frag app ui report">
      <span><span class="tag new">${report.tag}</span></span>
      <div class="h1 h1-sm">${report.title}</div>
      <span class="muted small">${report.meta}</span>
      <div class="card feedback"><span><span class="tag">${report.effort}</span></span><p>${report.comment}</p></div>
      <div class="section-title">${report.section}</div>
      <div class="rows"><div class="row wrap"><span class="ex-name full">${report.exercise} <span class="muted small">· ${report.side}</span></span><span class="result">${report.done}</span><span class="result warn">${report.off}</span></div></div>
    </div>`,
  ]
}

function coach(t: Strings): string {
  const pieces = fragments(t)
  const steps = t.coach.steps
    .map(
      (step, index) => `
    <li class="step">
      <span class="step-number">0${index + 1}</span>
      <div class="step-text"><h3>${step.title}</h3><p>${step.text}</p></div>
      <div class="step-frag" aria-hidden="true">${pieces[index] ?? ''}</div>
    </li>`,
    )
    .join('')
  return `
<section class="section" id="coach">
  <div class="section-head"><span class="label">${t.coach.label}</span><h2>${t.coach.title}</h2></div>
  <ol class="steps">${steps}</ol>
</section>`
}

function clients(t: Strings): string {
  const x = t.exercise
  const rows = x.rows
    .map(
      (row, index) => `
      <div class="set-row${row.done ? ' done' : ''}"><span class="muted">${index + 1}</span><span class="muted">${row.plan}</span><span class="field">${row.kg}</span><span class="field">${row.reps}</span><span class="tick${row.done ? ' ok' : ''}"></span></div>`,
    )
    .join('')
  const screen = `
    <div class="topbar"><span class="back">‹ ${t.builder.title}</span></div>
    <div class="video"><span class="play"></span></div>
    <div class="exercise-head"><span class="muted small">${x.count}</span><div class="h1">${x.name}</div><span class="muted small">${x.sets}</span></div>
    <p class="last-time">${x.last} <strong>${x.lastSets}</strong></p>
    <div class="set-table">
      <div class="set-row head"><span>${x.head[0]}</span><span>${x.head[1]}</span><span>${x.head[2]}</span><span>${x.head[3]}</span><span></span></div>
      ${rows}
    </div>
    <span class="muted small">${x.progress}</span>
    <div class="exercise-nav"><span class="btn-app">${x.all}</span><span class="btn-app primary">${x.next} ›</span></div>`
  const points = t.clients.points
    .map((point) => `<li><h3>${point.title}</h3><p>${point.text}</p></li>`)
    .join('')
  return `
<section class="section clients" id="clients">
  <div class="clients-phone">${phone(screen, x.name)}</div>
  <div class="clients-text">
    <div class="section-head"><span class="label">${t.clients.label}</span><h2>${t.clients.title}</h2></div>
    <ul class="points">${points}</ul>
  </div>
</section>`
}

function more(t: Strings): string {
  const items = t.more.items.map((item) => `<li><h3>${item.title}</h3><p>${item.text}</p></li>`).join('')
  return `
<section class="section more">
  <span class="label">${t.more.label}</span>
  <ul class="grid">${items}</ul>
</section>`
}

function beta(t: Strings): string {
  return `
<section class="section beta" id="beta">
  <h2>${t.beta.title}</h2>
  <p>${t.beta.text}</p>
  ${contact(t.beta.cta, 'big')}
</section>`
}

function faq(t: Strings): string {
  const items = t.faq.items
    .map((item) => `<details><summary>${item.q}</summary><p>${item.a}</p></details>`)
    .join('')
  return `
<section class="section faq" id="faq">
  <span class="label">${t.faq.label}</span>
  <div class="faq-list">${items}</div>
</section>`
}

function languages(locale: Locale): string {
  return `<span class="lang">${(['uk', 'en'] as const)
    .map((other) =>
      other === locale
        ? `<span class="on" aria-current="page">${other === 'uk' ? 'UA' : 'EN'}</span>`
        : `<a href="${PATHS[other]}" hreflang="${other}" lang="${other}">${other === 'uk' ? 'UA' : 'EN'}</a>`,
    )
    .join('')}</span>`
}

export function renderPage(locale: Locale, assets: Assets): string {
  const t = STRINGS[locale]
  const url = ORIGIN + PATHS[locale]
  return `<!doctype html>
<html lang="${t.lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
<meta name="color-scheme" content="dark" />
<meta name="theme-color" content="#0e0d0d" />
<title>${t.title}</title>
<meta name="description" content="${t.description}" />
<link rel="canonical" href="${url}" />
<link rel="alternate" hreflang="uk" href="${ORIGIN}${PATHS.uk}" />
<link rel="alternate" hreflang="en" href="${ORIGIN}${PATHS.en}" />
<link rel="alternate" hreflang="x-default" href="${ORIGIN}${PATHS.uk}" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="Coachin" />
<meta property="og:locale" content="${t.ogLocale}" />
<meta property="og:url" content="${url}" />
<meta property="og:title" content="${t.title}" />
<meta property="og:description" content="${t.description}" />
<meta property="og:image" content="${ORIGIN}/og-${locale}.png" />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta name="twitter:card" content="summary_large_image" />
<link rel="icon" href="/icon.svg" type="image/svg+xml" />
<link rel="apple-touch-icon" href="/apple-touch-icon.png" />
${assets.head}
</head>
<body>
<div class="page">
  <header class="nav">
    <a class="wordmark" href="${PATHS[locale]}">Coachin</a>
    <nav class="links" aria-label="Coachin"><a href="#demo">${t.nav.demo}</a><a href="#coach">${t.nav.coach}</a><a href="#clients">${t.nav.clients}</a><a href="#beta">${t.nav.beta}</a></nav>
    <div class="nav-end">
      ${languages(locale)}
      <a class="quiet client-link" href="/app">${t.nav.client} →</a>
      <a class="btn nav-cta" href="${CONTACT_URL}" target="_blank" rel="noopener">${t.nav.write}</a>
    </div>
  </header>
  <main>
    ${hero(t)}
    ${demo(t)}
    ${coach(t)}
    ${clients(t)}
    ${more(t)}
    ${beta(t)}
    ${faq(t)}
  </main>
  <footer class="footer">
    <span class="wordmark">Coachin</span>
    <a href="/app">${t.footer.client}</a>
    <a href="/install">${t.footer.install}</a>
    ${languages(locale)}
    <span class="muted">© 2026</span>
  </footer>
</div>
${assets.scripts}
</body>
</html>
`
}
