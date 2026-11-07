// Shows that the projection tests and the surface tests can fail: the code is broken in fifteen small ways, one at a time, and a test has
// to notice each. A test that passes proves nothing until it is known to fail on a fault.
//
//   node dess/s2/mutation-check.mjs [--out FILE]
//
// Each mutation is a single edit to a source file of `service-sim`, made on a copy that is put back afterwards, even if the run is
// interrupted. The browser mutations run `wasm-pack test --headless --chrome service-sim` and need Chrome, as the tests do. The engine mutations run the engine's own tests.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null

const SURFACE = 'service-sim/src/surface.rs'
const PROJECTION = 'service-sim/src/projection/mod.rs'
const ENGINE = 'service-sim-engine/src/engine.rs'

const MUTATIONS = [
  [
    SURFACE,
    'a hidden slot is not marked hidden',
    '.set_attribute("aria-hidden", "true")',
    '.set_attribute("aria-hidden", "false")',
    'browser'
  ],
  [
    SURFACE,
    'the data node is given zeros and not the values',
    'NodeValues::U32(projection.data.to_vec())',
    'NodeValues::U32(vec![0; DATA_CELLS])',
    'browser'
  ],
  [
    SURFACE,
    'a delta writes a blank class',
    'el.group.set_attribute("class", &slot.class)',
    'el.group.set_attribute("class", "")',
    'browser'
  ],
  [
    SURFACE,
    'a failed draw keeps trusting the page',
    'let old = self.drawn.take();',
    'let old = self.drawn.clone();',
    'browser'
  ],
  [
    SURFACE,
    'the host is not checked between writes',
    'for (index, w) in list.iter().enumerate() {\n            #[cfg(not(feature = "fault-hook"))]\n            let _ = index;\n            self.require_connected()?;',
    'for (index, w) in list.iter().enumerate() {\n            #[cfg(not(feature = "fault-hook"))]\n            let _ = index;',
    'browser'
  ],
  [
    SURFACE,
    'the host is not checked before the setter',
    '        self.require_connected()?;\n        self.scene',
    '        self.scene',
    'browser'
  ],
  [
    SURFACE,
    'the host is not checked after the setter',
    'self.fire(FaultPoint::Setter);\n        self.require_connected()?;',
    'self.fire(FaultPoint::Setter);',
    'browser'
  ],
  [
    SURFACE,
    'the more line is drawn at the top of its row',
    'let (x, y) = more_at();\n            (x, y + 17.0)',
    'let (x, y) = more_at();\n            (x, y)',
    'browser'
  ],
  [
    ENGINE,
    'the retry-wait count is not lowered when a retry re-queues a job',
    '                self.retry_wait -= 1;\n                self.jobs[id].entered = tick;',
    '                self.jobs[id].entered = tick;',
    'engine'
  ],
  [
    ENGINE,
    'the retry-wait count is not lowered when a job is finished',
    'if was_waiting_to_retry {\n            self.retry_wait -= 1;\n        }',
    '',
    'engine'
  ],
  [
    ENGINE,
    'a model copies every job that has arrived',
    'let mut ids = waiting.clone();',
    'let mut ids: Vec<u64> = (0..self.jobs.len() as u64).collect();',
    'engine'
  ],
  [
    PROJECTION,
    'the tick in the data node is cut to a byte',
    'clamp(model.tick),',
    'clamp(model.tick & 0xFF),',
    'browser'
  ],
  [
    PROJECTION,
    'the queue window is shifted by one',
    'model.waiting.get(position)',
    'model.waiting.get(position + 1)',
    'native'
  ],
  [
    PROJECTION,
    'a delta never writes a changed label',
    'differs(|a, b| a.label == b.label)',
    'differs(|_, _| false)',
    'native'
  ],
  [PROJECTION, 'the stale count is not shown', 'c.stale,\n        c.noops,', '0,\n        c.noops,', 'native']
]

const originals = new Map(MUTATIONS.map(([file]) => [file, fs.readFileSync(path.join(root, file), 'utf8')]))
// Every mutation has to apply to the source as it is now, and rustfmt can move the text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([file, , from]) => !originals.get(file).includes(from)).map(([, name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const COMMANDS = {
  browser: ['wasm-pack', 'test', '--headless', '--chrome', 'service-sim'],
  native: ['cargo', 'test', '-p', 'service-sim', '--lib'],
  engine: ['cargo', 'test', '-p', 'service-sim-engine', '--test', 'known_answers']
}

const results = []
try {
  for (const [file, name, from, to, kind] of MUTATIONS) {
    fs.writeFileSync(path.join(root, file), originals.get(file).replace(from, to))
    const args = COMMANDS[kind]
    const run = spawnSync(args[0], args.slice(1), { cwd: root, encoding: 'utf8', timeout: 900_000 })
    fs.writeFileSync(path.join(root, file), originals.get(file))
    const text = `${run.stdout}${run.stderr}`
    const failed = text
      .split('\n')
      .filter(line => line.startsWith('test ') && (line.endsWith(' FAILED') || line.endsWith(' FAIL')))
      .map(line => line.slice(5).split(' ... ')[0])
    const caught = failed.length > 0
    results.push({ name, kind, caught, failed })
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}`)
    if (caught) console.log(`         first test to fail: ${failed[0]}`)
  }
} finally {
  for (const [file, text] of originals) fs.writeFileSync(path.join(root, file), text)
}

const missed = results.filter(r => !r.caught)
console.log(`${results.length - missed.length} of ${results.length} mutations were caught, and the source is restored`)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify({ mutations: results }, null, 2)}\n`)
process.exit(missed.length === 0 ? 0 : 1)
