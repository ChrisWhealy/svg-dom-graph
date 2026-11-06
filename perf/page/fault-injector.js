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
;(() => {
  if (window.__faults) return

  const LABEL = 'data-svg-dom-graph-edge-label'
  let recording = false
  let calls = []
  let armed = null

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
    if (armed && armed.fired === null) {
      armed.counted++
      if (armed.counted === armed.index) {
        armed.fired = call
        throw new DOMException(`an injected failure of call ${armed.index}, ${method}`, 'InvalidStateError')
      }
    } else if (armed) {
      armed.counted++
    }
  }

  const wrap = (prototype, method) => {
    const native = prototype[method]
    prototype[method] = function (...args) {
      count(method, this, args)
      return native.apply(this, args)
    }
  }
  wrap(Element.prototype, 'setAttribute')
  wrap(Element.prototype, 'setAttributeNS')
  wrap(Node.prototype, 'appendChild')
  wrap(Node.prototype, 'insertBefore')
  wrap(Document.prototype, 'createElementNS')
  wrap(Document.prototype, 'createElement')
  wrap(EventTarget.prototype, 'addEventListener')
  wrap(SVGGraphicsElement.prototype, 'getBBox')

  // For the self-test only: removal that does nothing, which is a library with no rollback. If the sweeps cannot tell, they prove
  // nothing about the rollback that the library has.
  let suppressRemoval = false
  const nativeRemove = Element.prototype.remove
  Element.prototype.remove = function () {
    if (suppressRemoval) return
    return nativeRemove.call(this)
  }

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

  window.__faults = {
    record: on => {
      recording = on
      if (on) calls = []
    },
    take: () => {
      const taken = calls
      calls = []
      return taken
    },
    arm: index => {
      armed = { index, counted: 0, fired: null }
    },
    // Gives how many calls were counted while armed, and the one that failed, or null if fewer than `index` were made.
    suppressRemoval: on => {
      suppressRemoval = on
    },
    disarm: () => {
      const result = armed ? { counted: armed.counted, fired: armed.fired } : null
      armed = null
      return result
    }
  }
})()
