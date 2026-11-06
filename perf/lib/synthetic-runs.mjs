// Runs of a metric that are written to a directory in the shape of real results, with timings drawn from a fixed seed. They let the
// comparison and the promotion be tested, and measured, without a browser.
import fs from 'node:fs'
import path from 'node:path'
import { run } from './baselines.mjs'

export const HEAD = run('git', ['rev-parse', 'HEAD']).stdout.trim()
export const OTHER = 'f'.repeat(40)

// A small deterministic source of noise.
export const generator = seed => {
  let state = seed >>> 0
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state / 2 ** 32 - 0.5
  }
}

/**
 * Writes the runs of one collection to `dir`: for each, a result and a samples file. `centre` is the typical time of one
 * observation, `shift` how far a whole run sits from it, `noise` how far one observation sits from its run.
 */
export const collection = (dir, name, { runs = 5, centre = 50, shift = 0.01, noise = 0.05, seed = 1, ...overrides } = {}) => {
  fs.mkdirSync(dir, { recursive: true })
  const random = generator(seed)
  for (let repetition = 0; repetition < runs; repetition++) {
    const runId = `${name}-B-performance-r${repetition}`
    const level = centre * (1 + shift * random() * 2)
    const observations = Array.from({ length: 60 }, () => level * (1 + noise * random() * 2))
    const samples = `${runId}.samples.ndjson`
    fs.writeFileSync(path.join(dir, samples), JSON.stringify({ metric: 'mutation.move_node', values: observations }) + '\n')
    const doc = {
      schema_version: 1,
      run_id: runId,
      repetition,
      environment: {
        kind: 'browser',
        browser: 'Chrome',
        browser_version: '157.0.7632.0',
        os: 'Linux 6.8',
        cpu: 'Example CPU',
        runner_class: 'ci-linux',
        protocol_version: '1',
        git_sha: HEAD,
        git_dirty: false,
        ...overrides.environment
      },
      manifest: { id: 'w2-connector-rerouting-smoke', sha256: 'a'.repeat(64), operation_stream_sha256: 'b'.repeat(64), ...overrides.manifest },
      mode: 'performance',
      status: 'ok',
      metrics: [{ name: 'mutation.move_node', layer: 'B', status: 'measured' }],
      raw_samples: `target/perf/${samples}`,
      ...overrides.document
    }
    fs.writeFileSync(path.join(dir, `${runId}.json`), JSON.stringify(doc, null, 2) + '\n')
  }
  return dir
}

