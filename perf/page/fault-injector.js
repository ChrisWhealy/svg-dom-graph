// Makes the Nth DOM call of a page fail, on purpose, so that a failure can be put at every place an operation can have one.
// Inject it before any page script runs, as the listener tracker is.
//
// What counts as a call is a write or a measurement that the library makes through the DOM and that can fail: setting an
// attribute, creating or inserting an element, registering a listener, and measuring a box. Only a call that the bindings
// declare as able to fail is counted. Setting text, reading a bounding rectangle and measuring a run of text return no error in
// `web-sys`, so a failure there could only escape as an exception through the wasm frames, which is not a failure the library
// can handle and not one it could have. Removing an element is not counted either, because the library's own rollback removes
// elements, and a failure of the rollback is a different experiment from a failure that is rolled back. The Nth counted call after `arm(N)` throws a DOMException, which the wasm glue turns into the
// error that the library then handles as it would a real one. It fires once, and then everything works again.
//
//   __faults.record(true); ...; __faults.take()   describes every counted call that was made, in order
//   __faults.arm(n); ...; __faults.disarm()       fails the nth counted call; gives how many were counted and which one failed
//
// A second mode puts a host in the document one write short of what a draw needs, and it is separate from the first. It is for the simulator's gallery
// panel (design-specs/dess_demo.pdf, P0-F), whose draw has to meet a host that leaves in the middle of it:
//
//   __faults.armDetach({ svgId, method, nth })    the nth counted call of `method` on a node inside `#svgId` is made natively, and then the
//                                                 `<svg>` is removed from the document before control returns to the caller
//   __faults.disarmDetach()                       says what happened: how many eligible calls were seen, whether it fired, and at which call
//   __faults.reattach()                           puts the `<svg>` back where it was, so that a repair can be made
//
// It is off until it is armed, and it is one-shot. It never changes what `record`, `arm` and `take` count or when: its own count is kept apart, and
// it only looks at a call after the call has been counted and made. It will not be armed while `arm` is, since a call is not both failed and
// detached on. A detach that was armed and never fired is reported as not fired, and a test has to treat that as a failure and not as a pass.
;(() => {
  if (window.__faults) return

  const LABEL = 'data-svg-dom-graph-edge-label'
  let recording = false
  let calls = []
  let armed = null
  // The detach mode, which is a separate state from `armed` and is null until armDetach is called.
  let detach = null
  let detached = null

  const describe = (method, target, args) => {
    const named =
      method === 'setAttribute'
        ? args[0]
        : method === 'setAttributeNS'
          ? args[1]
          : method === 'addEventListener'
            ? args[0]
            : undefined
    const tag =
      method === 'createElementNS' ? args[1] : method === 'createElement' ? args[0] : (target?.localName ?? '')
    return { method, tag, name: typeof named === 'string' ? named : '', label: Boolean(target?.hasAttribute?.(LABEL)) }
  }

  const count = (method, target, args) => {
    if (!recording && !armed) return
    const call = describe(method, target, args)
    if (recording) calls.push(call)
    if (armed) {
      armed.counted++
      // The call that is armed fails, and with `times` more than one so do the calls straight after it, which is how a recovery that
      // itself meets a failure is tested. Only the first call that failed is reported.
      if (armed.counted >= armed.index && armed.counted < armed.index + armed.times) {
        if (armed.fired === null) armed.fired = call
        armed.all.push(call)
        throw new DOMException(`an injected failure of call ${armed.counted}, ${method}`, 'InvalidStateError')
      }
    }
  }

  // After a call has been counted and made: whether it is the one that the detach was armed for. It does not touch the counting above.
  const afterCall = (method, target, args) => {
    if (detach.fired || method !== detach.method || !(target instanceof Node) || !detach.svg.contains(target)) return
    detach.seen++
    if (detach.seen !== detach.nth) return
    detach.fired = true
    detach.firings++
    const described = describe(method, target, args)
    detach.site = { method, tag: described.tag, name: described.name, ordinal: detach.seen }
    // The host leaves as a person's would: removed from where it is, and the position kept so that it can be put back.
    detached = { parent: detach.svg.parentNode, next: detach.svg.nextSibling, svg: detach.svg }
    nativeRemove.call(detach.svg)
  }

  const wrap = (prototype, method) => {
    const native = prototype[method]
    prototype[method] = function (...args) {
      count(method, this, args)
      const result = native.apply(this, args)
      if (detach !== null) afterCall(method, this, args)
      return result
    }
  }
  // The native removal, taken before anything is wrapped, which the detach uses and which the self-test's removal that does nothing does not affect.
  let suppressRemoval = false
  const nativeRemove = Element.prototype.remove
  const nativeInsertBefore = Node.prototype.insertBefore
  wrap(Element.prototype, 'setAttribute')
  wrap(Element.prototype, 'setAttributeNS')
  wrap(Node.prototype, 'appendChild')
  wrap(Node.prototype, 'insertBefore')
  wrap(Document.prototype, 'createElementNS')
  wrap(Document.prototype, 'createElement')
  wrap(EventTarget.prototype, 'addEventListener')
  wrap(SVGGraphicsElement.prototype, 'getBBox')

  // Animation frames, which a page can be made to refuse, or to hold back so that something can happen while one is pending.
  //   __frames.refuseNext()   the next requestAnimationFrame throws, as a browser that will not give a frame would
  //   __frames.hold(true)     requested frames are kept and not run, and a cancelled one is taken out of the queue
  //   __frames.release()      runs the frames that are still held, and says how many ran and what any of them threw
  const nativeRequest = window.requestAnimationFrame.bind(window)
  const nativeCancel = window.cancelAnimationFrame.bind(window)
  const held = new Map()
  const cancelled = []
  let refuse = 0
  let holding = false
  let nextHeld = 1_000_000
  let suppressCancel = false
  window.requestAnimationFrame = callback => {
    if (refuse > 0) {
      refuse--
      throw new DOMException('an injected refusal of a frame', 'InvalidStateError')
    }
    if (holding) {
      held.set(nextHeld, callback)
      return nextHeld++
    }
    return nativeRequest(callback)
  }
  window.cancelAnimationFrame = id => {
    cancelled.push(id)
    if (suppressCancel) return
    if (held.delete(id)) return
    nativeCancel(id)
  }
  window.__frames = {
    refuseNext: () => {
      refuse = 1
    },
    hold: on => {
      holding = on
    },
    pending: () => held.size,
    cancelled: () => cancelled.length,
    // For the self-test only: a cancel that does not cancel, which is a library that leaves a frame pending as its closure goes.
    suppressCancel: on => {
      suppressCancel = on
    },
    clear: () => {
      cancelled.length = 0
      held.clear()
      refuse = 0
    },
    release: () => {
      holding = false
      const callbacks = [...held.values()]
      held.clear()
      const threw = []
      for (const callback of callbacks) {
        try {
          callback(performance.now())
        } catch (error) {
          threw.push(String(error?.message ?? error))
        }
      }
      return { ran: callbacks.length, threw }
    }
  }

  // Timers, which a page can be made to refuse, as a browser that will not give one would, or to hold, so that a test decides which runs
  // and when, and no wall-clock interval is waited for.
  //   __timers.refuseNext()   the next setTimeout throws
  //   __timers.hold(true)     requested timers are kept with their delay and not scheduled, and a cleared one is taken out of the queue
  //   __timers.pending()      the held timers, as { id, delay }, in the order that they were asked for
  //   __timers.fire(id)       runs one held timer now, and says what it threw, if anything
  // While timers are held a page's own `setTimeout` is held too, so a test does not wait on one.
  const nativeSetTimeout = window.setTimeout.bind(window)
  const nativeClearTimeout = window.clearTimeout.bind(window)
  const heldTimers = new Map()
  let holdingTimers = false
  let nextTimer = 2_000_000
  let refuseTimer = 0
  window.setTimeout = (callback, ...rest) => {
    if (refuseTimer > 0) {
      refuseTimer--
      throw new DOMException('an injected refusal of a timer', 'InvalidStateError')
    }
    if (holdingTimers) {
      heldTimers.set(nextTimer, { callback, delay: rest[0] ?? 0 })
      return nextTimer++
    }
    return nativeSetTimeout(callback, ...rest)
  }
  window.clearTimeout = id => {
    if (heldTimers.delete(id)) return
    nativeClearTimeout(id)
  }
  window.__timers = {
    refuseNext: () => {
      refuseTimer = 1
    },
    hold: on => {
      holdingTimers = on
    },
    pending: () => [...heldTimers].map(([id, { delay }]) => ({ id, delay })),
    fire: id => {
      const timer = heldTimers.get(id)
      if (!timer) return { ran: false, threw: null }
      heldTimers.delete(id)
      try {
        timer.callback()
        return { ran: true, threw: null }
      } catch (error) {
        return { ran: true, threw: String(error?.message ?? error) }
      }
    },
    clear: () => {
      refuseTimer = 0
      holdingTimers = false
      heldTimers.clear()
    }
  }

  // For the self-test only: removal that does nothing, which is a library with no rollback. If the sweeps cannot tell, they prove
  // nothing about the rollback that the library has.
  Element.prototype.remove = function () {
    if (suppressRemoval) return
    return nativeRemove.call(this)
  }

  window.__faults = {
    armDetach: ({ svgId, method = 'setAttribute', nth = 1 }) => {
      if (armed !== null)
        throw new Error('armDetach: an exception is armed, and a call is not both failed and detached on')
      if (detach !== null) throw new Error('armDetach: a detach is already armed')
      const svg = document.getElementById(svgId)
      if (!svg) throw new Error(`armDetach: there is no element #${svgId}`)
      if (!Number.isInteger(nth) || nth < 1) throw new Error('armDetach: nth is a whole number from 1')
      detach = { svg, svgId, method, nth, seen: 0, fired: false, firings: 0, site: null }
      detached = null
    },
    disarmDetach: () => {
      const report = detach
        ? {
            svgId: detach.svgId,
            method: detach.method,
            nth: detach.nth,
            eligibleSeen: detach.seen,
            fired: detach.fired,
            firings: detach.firings,
            site: detach.site,
            detached: detached !== null && !detached.svg.isConnected
          }
        : null
      detach = null
      return report
    },
    reattach: () => {
      if (!detached || detached.svg.isConnected) return false
      nativeInsertBefore.call(detached.parent, detached.svg, detached.next)
      detached = null
      return true
    },
    record: on => {
      recording = on
      if (on) calls = []
    },
    take: () => {
      const taken = calls
      calls = []
      return taken
    },
    arm: (index, times = 1) => {
      armed = { index, times, counted: 0, fired: null, all: [] }
    },
    // Gives how many calls were counted while armed, and the one that failed, or null if fewer than `index` were made.
    suppressRemoval: on => {
      suppressRemoval = on
    },
    disarm: () => {
      const result = armed ? { counted: armed.counted, fired: armed.fired, all: armed.all } : null
      armed = null
      return result
    }
  }
})()
