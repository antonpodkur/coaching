/** Fields whose caret the page can move; date and number inputs cannot. */
const TEXT_TYPES = new Set(['text', 'search', 'tel', 'url'])
/** Weights, reps, times and ranges: `27,5`, `12`, `1:30`, `8-10`. */
const NUMBER_LIKE = /^[\d\s.,:–-]+$/

/**
 * Where the caret goes when a field gets focus. Phones put it where the finger
 * landed, often before the text (`|10`). A number is selected whole instead, so
 * typing replaces it; in text the caret goes to the end, where one carries on.
 * Taps inside a field that already has focus move the caret as usual.
 */
export function placeCaretOnFocus() {
  document.addEventListener('focusin', (event) => {
    const field = event.target
    const editable =
      field instanceof HTMLTextAreaElement ||
      (field instanceof HTMLInputElement && TEXT_TYPES.has(field.type))
    if (!editable || field.readOnly) return

    const value = field.value
    const whole = NUMBER_LIKE.test(value)
    const place = () => {
      // Leave it alone once they are typing or have moved on.
      if (document.activeElement !== field || field.value !== value) return
      field.setSelectionRange(whole ? 0 : value.length, value.length)
    }
    place()
    // The tap that gave focus sets its own caret a moment later; win after it.
    setTimeout(place, 0)
    setTimeout(place, 100)

    // Releasing a mouse click would also collapse the selection.
    const done = new AbortController()
    field.addEventListener(
      'mouseup',
      (up) => {
        up.preventDefault()
        done.abort()
      },
      { signal: done.signal },
    )
    field.addEventListener('blur', () => done.abort(), { signal: done.signal })
  })
}
