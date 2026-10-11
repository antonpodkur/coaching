// The landing page's only script. The page is complete without it; this ties
// each exercise in the chat message to the card it became in the app.

import '@fontsource-variable/manrope'
import '@fontsource/unbounded/latin-600.css'
import '@fontsource/unbounded/cyrillic-600.css'
import './landing.css'

const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)')

/** One thread per exercise: a bracket over its lines, into its card. */
function drawThreads(animate: boolean) {
  const stage = document.getElementById('stage')
  const svg = document.getElementById('threads')
  if (!stage || !(svg instanceof SVGSVGElement) || getComputedStyle(svg).display === 'none') return

  const box = stage.getBoundingClientRect()
  const bubble = stage.querySelector('.bubble.out')?.getBoundingClientRect()
  if (!bubble) return
  svg.setAttribute('viewBox', `0 0 ${box.width} ${box.height}`)
  svg.replaceChildren()
  svg.classList.remove('drawing')

  const cards = [...stage.querySelectorAll<HTMLElement>('.card[data-g]')]
  const drawn = cards.flatMap((card) => {
    const lines = [...stage.querySelectorAll(`.l[data-g="${card.dataset.g}"]`)].map((line) =>
      line.getBoundingClientRect(),
    )
    if (lines.length === 0) return []
    const top = Math.min(...lines.map((r) => r.top)) - box.top + 4
    const bottom = Math.max(...lines.map((r) => r.bottom)) - box.top - 4
    const x1 = bubble.right - box.left + 10
    const y1 = (top + bottom) / 2
    const end = card.getBoundingClientRect()
    const x2 = end.left - box.left
    const y2 = end.top + end.height / 2 - box.top
    const dx = (x2 - x1) / 2

    const bracket = path(`M${x1 - 5} ${top} H${x1} V${bottom} H${x1 - 5}`, 'bracket')
    const thread = path(`M${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`)
    const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle')
    dot.setAttribute('cx', String(x2))
    dot.setAttribute('cy', String(y2))
    dot.setAttribute('r', '3')
    svg.append(bracket, thread, dot)
    return [{ parts: [bracket, thread], dot, card }]
  })

  if (!animate) return
  // Hidden first, then drawn in one exercise after another.
  for (const { parts, dot } of drawn) {
    for (const part of parts) {
      const length = part.getTotalLength()
      part.style.strokeDasharray = `${length}`
      part.style.strokeDashoffset = `${length}`
    }
    dot.style.opacity = '0'
  }
  svg.getBoundingClientRect()
  svg.classList.add('drawing')
  drawn.forEach(({ parts, dot, card }, index) => {
    const delay = 350 + index * 260
    for (const part of parts) {
      part.style.transitionDelay = `${delay}ms`
      part.style.strokeDashoffset = '0'
    }
    dot.style.transitionDelay = `${delay + 750}ms`
    dot.style.opacity = '1'
    window.setTimeout(() => card.classList.add('lit'), delay + 800)
    window.setTimeout(() => card.classList.remove('lit'), delay + 1700)
  })
}

function path(d: string, className?: string): SVGPathElement {
  const element = document.createElementNS('http://www.w3.org/2000/svg', 'path')
  element.setAttribute('d', d)
  if (className) element.setAttribute('class', className)
  return element
}

// Lines move when the fonts arrive, so wait for them before measuring.
void document.fonts.ready.then(() => drawThreads(!reduceMotion.matches))

let resizing = 0
window.addEventListener('resize', () => {
  cancelAnimationFrame(resizing)
  resizing = requestAnimationFrame(() => drawThreads(false))
})
