// A stand-in for the product's controller, surface and scheduler, for the S0 protocol spike. It is not the S2 or S3 code. It is
// shaped like design revision 6 section 6, so that what the harness has to do to it can be tried before the real one exists:
//
//   * time reaches the controller only through a scheduler with `schedule(callback, ms)` and `cancel(handle)`;
//   * Play does not advance at once. It schedules one timer, and the next one only after a commit;
//   * every stop advances an epoch, and a callback from an earlier epoch does nothing;
//   * a commit draws, and only then moves the cursor.
//
// There is no measurement code in this file. That is the point of the spike: the harness composes this file with a scheduler of its
// own, and writes its marks from there. `protocol-spike.mjs` checks that this file never names a mark or the harness.

export class BrowserScheduler {
  constructor() {
    this.live = new Map()
    this.next = 1
    // The test mode of the design: a cancelled callback is kept, and can be made to run, as a browser sometimes lets one.
    this.keepCancelled = false
    this.held = []
  }
  schedule(callback, ms) {
    const handle = this.next++
    const timeout = setTimeout(() => {
      this.live.delete(handle)
      callback()
    }, ms)
    this.live.set(handle, { timeout, callback })
    return handle
  }
  cancel(handle) {
    const entry = this.live.get(handle)
    if (!entry) return
    clearTimeout(entry.timeout)
    this.live.delete(handle)
    if (this.keepCancelled) this.held.push(entry.callback)
  }
  /** Runs every cancelled callback that was kept, oldest first. */
  runHeld() {
    const callbacks = this.held.splice(0)
    for (const callback of callbacks) callback()
    return callbacks.length
  }
  get pending() {
    return this.live.size
  }
}

const SVG_NS = 'http://www.w3.org/2000/svg'

/** Eight counter cells and twelve queue slots, drawn once and then only written, as the fixed-slot design has it. */
export class Surface {
  constructor(host) {
    this.host = host
    this.cells = []
    this.slots = []
    this.status = null
    const group = document.createElementNS(SVG_NS, 'g')
    group.setAttribute('class', 'svg-dom-graph-content')
    host.append(group)
    for (let i = 0; i < 8; i++) {
      const text = document.createElementNS(SVG_NS, 'text')
      text.setAttribute('x', String(10 + (i % 4) * 70))
      text.setAttribute('y', String(30 + Math.floor(i / 4) * 24))
      text.setAttribute('aria-label', `cell ${i}`)
      text.textContent = '0'
      group.append(text)
      this.cells.push(text)
    }
    for (let i = 0; i < 12; i++) {
      const rect = document.createElementNS(SVG_NS, 'rect')
      rect.setAttribute('x', String(10 + i * 22))
      rect.setAttribute('y', '90')
      rect.setAttribute('width', '18')
      rect.setAttribute('height', '18')
      rect.setAttribute('class', 'slot')
      group.append(rect)
      this.slots.push(rect)
    }
    this.status = document.createElementNS(SVG_NS, 'text')
    this.status.setAttribute('x', '10')
    this.status.setAttribute('y', '130')
    group.append(this.status)
  }

  /** The writes of one commit. Every commit changes the slice number, so that a slice that changed nothing else still draws. */
  draw(slice) {
    const values = [
      slice.index,
      slice.tick,
      slice.pops,
      slice.waiting,
      slice.busy,
      slice.waiting * 10,
      slice.busy * 10,
      slice.index % 97
    ]
    for (let i = 0; i < 8; i++) this.cells[i].textContent = String(values[i])
    for (let i = 0; i < 12; i++) this.slots[i].setAttribute('class', i < slice.waiting ? 'slot active' : 'slot')
    this.status.textContent = `slice ${slice.index}, tick ${slice.tick}`
  }
}

export class Controller {
  constructor({ slices, surface, scheduler, intervalMs = 1000 }) {
    this.slices = slices
    this.surface = surface
    this.scheduler = scheduler
    this.intervalMs = intervalMs
    this.cursor = 0
    this.epoch = 0
    this.playing = false
    this.timer = null
    this.timerId = 0
    this.armed = 0
    this.disposed = false
    this.commits = 0
    this.ticks = 0
  }

  commit(index) {
    if (this.disposed || !this.surface.host.isConnected || index >= this.slices.length) return false
    this.surface.draw(this.slices[index])
    this.cursor = index
    this.commits++
    return true
  }

  next() {
    this.stop()
    this.commit(this.cursor + 1)
  }

  play() {
    if (this.playing || this.disposed) return
    this.playing = true
    this.arm()
  }

  pause() {
    this.stop()
  }

  stop() {
    if (this.timer !== null) this.scheduler.cancel(this.timer)
    this.timer = null
    this.playing = false
    this.epoch++
  }

  arm() {
    const epoch = this.epoch
    // Each timer has an identity of its own, so that a callback can tell whether it is the timer that is pending now.
    const id = ++this.armed
    this.timerId = id
    this.timer = this.scheduler.schedule(() => this.onTick(epoch, id), this.intervalMs)
  }

  onTick(epoch, id) {
    // A callback of an earlier playback, or of a timer that was cancelled, changes nothing. In particular it must not clear the
    // handle of the timer that is pending now, or Pause could no longer cancel that one.
    if (this.disposed || epoch !== this.epoch || id !== this.timerId) return
    this.timer = null
    this.ticks++
    if (this.commit(this.cursor + 1) && this.playing) this.arm()
  }
}
