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
// To know that a `once` listener fired, and that a signal aborted, the tracker adds a small helper listener of its own
// beside the original. The helpers go through the native methods, so they are never counted. If a `once` listener calls
// `stopImmediatePropagation` itself, its helper does not fire and the registration is counted for too long. Checking the
// tally against Chrome's own count after a collection exposes that case.
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
      record(op, entry.ref.deref() ?? window, { type: registration.type, capture: registration.capture, born: registration.sequence })
    }
  }

  EventTarget.prototype.addEventListener = function (type, callback, options) {
    const result = nativeAdd.call(this, type, callback, options)
    if (callback == null) {
      record('ignored-null', this, { type })
      return result
    }
    const capture = captureOf(options)
    const once = Boolean(options && typeof options === 'object' && options.once)
    const signal = options && typeof options === 'object' ? options.signal : undefined
    if (signal && signal.aborted) {
      record('ignored-aborted', this, { type, capture })
      return result
    }
    const entry = entryFor(this)
    const key = keyOf(type, callback, capture)
    if (entry.registrations.has(key)) {
      record('duplicate', this, { type, capture })
      return result
    }
    const registration = { type, capture, once, sequence: sequence++ }
    entry.registrations.set(key, registration)
    record('add', this, { type, capture, once, born: registration.sequence })
    if (once) nativeAdd.call(this, type, () => drop(entry, key, registration, 'once-fired'), { capture, once: true })
    if (signal) nativeAdd.call(signal, 'abort', () => drop(entry, key, registration, 'aborted'), { once: true })
    return result
  }

  EventTarget.prototype.removeEventListener = function (type, callback, options) {
    const result = nativeRemove.call(this, type, callback, options)
    if (callback == null) return result
    const entry = entries.get(this)
    if (!entry) {
      record('remove-unknown', this, { type, capture: captureOf(options) })
      return result
    }
    const capture = captureOf(options)
    const key = keyOf(type, callback, capture)
    const registration = entry.registrations.get(key)
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
    stopLogging: () => (logging = false),
  }
})()
