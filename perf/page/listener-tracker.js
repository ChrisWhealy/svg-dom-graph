// Tracks the effective event-listener registrations of a page. Inject it before any page script runs.
//
// It follows the DOM rules for "add an event listener", not a count of calls. A registration is identified by its
// target, its type, its callback and its capture flag. Adding an identical one again changes nothing, and removing needs
// the same capture flag to match. A `once` listener stops counting after it fires. A listener whose `signal` aborts stops
// counting, and one whose `signal` is already aborted is never counted.
//
// The browser's own listeners are not seen, only those added through `addEventListener`. Property handlers such as
// `onclick` are not seen either, and Phase 0 found that the library uses none.
//
// A `once` listener is registered through a thin wrapper that calls the original and then records that it has fired. The
// wrapper runs whenever the original would, so a listener that calls `stopImmediatePropagation` on itself is still seen.
// A signal is checked when the tally is read and when an identical listener is added again, so nothing has to listen for
// the abort. The wrapper is the only trace the tracker leaves on a listener, and DevTools shows it in place of the original.
;(() => {
  if (window.__listeners) return

  const nativeAdd = EventTarget.prototype.addEventListener
  const nativeRemove = EventTarget.prototype.removeEventListener

  const callbackIds = new WeakMap()
  let nextCallbackId = 0
  const entries = new WeakMap()
  const live = new Set()
  let sequence = 0
  let logging = false
  const log = []

  const idOf = callback => {
    if (!callbackIds.has(callback)) callbackIds.set(callback, nextCallbackId++)
    return callbackIds.get(callback)
  }
  const captureOf = options => (typeof options === 'boolean' ? options : Boolean(options && options.capture))
  const keyOf = (type, callback, capture) => `${type}|${capture}|${idOf(callback)}`

  const entryFor = target => {
    let entry = entries.get(target)
    if (!entry) {
      entry = { ref: new WeakRef(target), registrations: new Map() }
      entries.set(target, entry)
      live.add(entry)
    }
    return entry
  }

  const kindOf = target => {
    if (target === window) return 'window'
    if (target instanceof Document) return 'document'
    if (target instanceof Element) return target.localName
    return target.constructor ? target.constructor.name : 'unknown'
  }

  const record = (op, target, fields) => {
    if (!logging) return
    log.push({ op, kind: kindOf(target), ...fields, stack: new Error().stack.split('\n').slice(3, 9).join('\n') })
  }

  const drop = (entry, key, registration, op) => {
    // A listener added again after it was removed is a new registration, which this one must not remove.
    if (entry.registrations.get(key) === registration) {
      entry.registrations.delete(key)
      record(op, entry.ref.deref() ?? window, {
        type: registration.type,
        capture: registration.capture,
        born: registration.sequence
      })
    }
  }

  // The registration for `key`, or undefined. One whose signal has aborted is already gone from the browser, so it is
  // removed here, and an identical listener can then be added again.
  const liveRegistration = (entry, key) => {
    const registration = entry.registrations.get(key)
    if (registration && registration.signal && registration.signal.aborted) {
      drop(entry, key, registration, 'aborted')
      return undefined
    }
    return registration
  }

  EventTarget.prototype.addEventListener = function (type, callback, options) {
    if (callback == null) {
      record('ignored-null', this, { type })
      return nativeAdd.call(this, type, callback, options)
    }
    const capture = captureOf(options)
    const object = options && typeof options === 'object'
    const once = Boolean(object && options.once)
    const signal = object ? options.signal : undefined
    if (signal && signal.aborted) {
      record('ignored-aborted', this, { type, capture })
      return nativeAdd.call(this, type, callback, options)
    }
    const entry = entryFor(this)
    const key = keyOf(type, callback, capture)
    if (liveRegistration(entry, key)) {
      record('duplicate', this, { type, capture })
      return nativeAdd.call(this, type, callback, options)
    }
    const registration = { type, capture, once, signal, sequence: sequence++, wrapper: undefined }
    let native = callback
    if (once) {
      // The native listener is the wrapper, so that firing is observed even if the original stops propagation.
      registration.wrapper = function (event) {
        try {
          if (typeof callback === 'function') return callback.call(this, event)
          return callback.handleEvent(event)
        } finally {
          drop(entry, key, registration, 'once-fired')
        }
      }
      native = registration.wrapper
    }
    // The browser validates the callback and the options, and throws before anything is recorded.
    const result = nativeAdd.call(this, type, native, options)
    entry.registrations.set(key, registration)
    record('add', this, { type, capture, once, born: registration.sequence })
    return result
  }

  EventTarget.prototype.removeEventListener = function (type, callback, options) {
    if (callback == null) return nativeRemove.call(this, type, callback, options)
    const entry = entries.get(this)
    const capture = captureOf(options)
    const key = keyOf(type, callback, capture)
    const registration = entry && liveRegistration(entry, key)
    // A wrapped listener is the one the browser holds, so that is the one to take out.
    const result = nativeRemove.call(
      this,
      type,
      registration && registration.wrapper ? registration.wrapper : callback,
      options
    )
    if (registration) drop(entry, key, registration, 'remove')
    else record('remove-unknown', this, { type, capture })
    return result
  }

  // `total` is every registration on a target that still exists. `connected` leaves out those on a node that is detached
  // from the document, which can only fire if something holds the node. Targets that are not nodes always count.
  const snapshot = () => {
    let total = 0
    let connected = 0
    const byKind = {}
    const byType = {}
    for (const entry of [...live]) {
      const target = entry.ref.deref()
      if (!target) {
        live.delete(entry)
        continue
      }
      for (const key of [...entry.registrations.keys()]) liveRegistration(entry, key)
      const count = entry.registrations.size
      if (!count) continue
      total += count
      if (!(target instanceof Node) || target.isConnected) connected += count
      byKind[kindOf(target)] = (byKind[kindOf(target)] ?? 0) + count
      for (const { type } of entry.registrations.values()) byType[type] = (byType[type] ?? 0) + 1
    }
    return { total, connected, detached: total - connected, byKind, byType }
  }

  window.__listeners = {
    snapshot,
    log,
    sequence: () => sequence,
    startLogging: () => (logging = true),
    stopLogging: () => (logging = false)
  }
})()
