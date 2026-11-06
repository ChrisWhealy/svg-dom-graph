// What `mixed_outage/7` is, pinned independently of what it qualifies.
//
// Two claims are separate. One is that the workload qualifies: enough seeds pass the activity bounds. The other is that the workload
// has not changed: the parameters, the input they generate and the events the engine pops for it are exactly what was frozen. A
// small change to a parameter could keep the first true and break the second, so `frozen.json` holds three hashes of the
// revision's manifest seed, and a check fails if any of them moves while the revision's name stays the same.
//
//   parameters_sha256     the parameter set, as canonical JSON (keys sorted, no spaces);
//   input_stream_sha256   everything the generator produces for the manifest seed, one line for each outage event and arrival;
//   event_pop_sha256      the engine's pop sequence for that input (tick, kind, insertion sequence and whether it was stale).
//
// A deliberate change is a new revision: edit the parameters, change `revision`, run the qualification panel, and write the new pins
// with `node dess/s0/prototype/freeze.mjs --write`.
import crypto from 'node:crypto'
import fs from 'node:fs'
import { run } from './engine.mjs'
import { MIXED_OUTAGE, configOf, inputsFor, makeOutages, makeSource } from './generator.mjs'

export const FROZEN_FILE = new URL('./frozen.json', import.meta.url)

const sha256 = text => crypto.createHash('sha256').update(text).digest('hex')

/** The same JSON whatever order the keys were written in. */
export const canonical = value => {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value && typeof value === 'object')
    return `{${Object.keys(value)
      .sort()
      .map(k => `${JSON.stringify(k)}:${canonical(value[k])}`)
      .join(',')}}`
  return JSON.stringify(value)
}

export const parametersHash = params => sha256(canonical(params))

/** The input of one seed: its outages, then every arrival's draws, in order. */
export const inputStream = (seed, params) => {
  const lines = makeOutages(seed, params).map(e => `${e.kind},${e.at},${e.server}`)
  const source = makeSource(seed, params)
  for (let i = 0; i < source.count; i++) {
    const s = source.next()
    lines.push(`arrive,${i},${s.gap},${s.priority},${s.durations.join('+')},${s.slack},${s.cancelAfter}`)
  }
  return lines.join('\n')
}

export const inputStreamHash = (seed, params) => sha256(inputStream(seed, params))

export const eventPopHash = (seed, params) => {
  const result = run(configOf(params), inputsFor(seed, params), { hash: crypto.createHash('sha256') })
  return { hash: result.hash, counters: result.counters }
}

/** The pins of a revision, computed from its parameters and manifest seed. */
export const computePins = (params, seed) => {
  const pops = eventPopHash(seed, params)
  return {
    revision: params.revision,
    manifest_seed: seed,
    parameters_sha256: parametersHash(params),
    input_stream_sha256: inputStreamHash(seed, params),
    event_pop_sha256: pops.hash,
    manifest_seed_run: { pops: pops.counters.pops, slices: pops.counters.slices, arrivals: pops.counters.arrivals }
  }
}

export const readFrozen = () => JSON.parse(fs.readFileSync(FROZEN_FILE, 'utf8'))

/**
 * Checks `params` against the pins. Parameters that carry another revision's name are a candidate and are not checked. Parameters
 * that carry the frozen revision's name have to be the frozen ones, or the check says what moved.
 */
export const checkPins = (params, frozen = readFrozen()) => {
  if (params.revision !== frozen.revision) return { checked: false, ok: true, problems: [] }
  const now = computePins(params, frozen.manifest_seed)
  const moved = ['parameters_sha256', 'input_stream_sha256', 'event_pop_sha256'].filter(k => now[k] !== frozen[k])
  const problems = moved.map(k => {
    const what = {
      parameters_sha256: 'the parameters',
      input_stream_sha256: `the input stream of seed ${frozen.manifest_seed}`,
      event_pop_sha256: `the event pops of seed ${frozen.manifest_seed}`
    }[k]
    return `${what} changed, and the revision is still ${frozen.revision}`
  })
  return { checked: true, ok: moved.length === 0, problems }
}

export { MIXED_OUTAGE }
