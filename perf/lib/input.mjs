// Real input, through `Input.dispatchMouseEvent`. The browser turns it into pointer and wheel events as it would for a person,
// so the page's listeners and the library's own handlers see what they would see in use. Nothing here calls into the page.

const CTRL = 2
export const pause = ms => new Promise(resolve => setTimeout(resolve, ms))

/** Sends one command and gives how long CDP took to answer it, in ms. That is the round trip, and is never a start time. */
const timed = async (page, params) => {
  const sent = performance.now()
  const reply = await page.send('Input.dispatchMouseEvent', params)
  if (reply.error) throw new Error(`Input.dispatchMouseEvent: ${reply.error.message}`)
  return performance.now() - sent
}

export const move = (page, x, y, { buttons = 0 } = {}) =>
  timed(page, { type: 'mouseMoved', x, y, button: buttons ? 'left' : 'none', buttons })
export const press = (page, x, y) =>
  timed(page, { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 })
export const release = (page, x, y) =>
  timed(page, { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 })

/** One notch of the wheel, with the control key held, which is how the library tells a zoom from a scroll. */
export const wheel = (page, x, y, deltaY, { ctrl = true } = {}) =>
  timed(page, { type: 'mouseWheel', x, y, deltaX: 0, deltaY, modifiers: ctrl ? CTRL : 0 })

/**
 * Presses at `from`, moves in `steps` equal moves to `from + (dx, dy)` with `gapMs` between them, and releases.
 *
 * Gives the round trip of every command. The points are rounded to whole pixels, as a pointer's are.
 */
export const dragPath = async (page, from, dx, dy, steps, gapMs) => {
  const trips = { press: 0, moves: [], release: 0 }
  await move(page, from[0], from[1])
  trips.press = await press(page, from[0], from[1])
  await pause(gapMs)
  for (let step = 1; step <= steps; step++) {
    trips.moves.push(
      await move(page, Math.round(from[0] + (dx * step) / steps), Math.round(from[1] + (dy * step) / steps), {
        buttons: 1
      })
    )
    await pause(gapMs)
  }
  trips.release = await release(page, Math.round(from[0] + dx), Math.round(from[1] + dy))
  return trips
}
