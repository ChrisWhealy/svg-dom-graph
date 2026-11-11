// P0-D: the demo build graph must not contain the simulator's test-only fault hook, and the check has to be able to tell.
//
//   node dess/demo-panel/p0/features.mjs [--require-service-sim] [--out dess/demo-panel/p0/evidence/features.txt]
//
// `service-sim` has a Cargo feature, `fault-hook`, that lets a test detach a host in the middle of a draw. It is the harness's and never the
// product's, and the gallery is one wasm build that must not have it. Cargo unifies a feature across the workspace members that are built together,
// so an unrelated change could switch it on in the demo without anyone meaning to. This reads the resolved feature graph of both builds:
//
//   cargo tree -p svg-dom-graph-demo   --target wasm32-unknown-unknown -e features    must NOT hold service-sim's "fault-hook"
//   cargo tree -p perf-harness-wasm    --target wasm32-unknown-unknown -e features    MUST hold it, which is the positive control
//
// A check that only ever sees nothing passes whatever the graph is, so the second command shows that the same reading finds the feature when it is
// there, and a parser sanity check shows that it finds the line in text that has it and not in text that does not.
//
// Until the gallery panel (P2) the demo does not depend on `service-sim` at all, so an absent feature is then true because the crate is absent. That is
// recorded as what it is. With `--require-service-sim`, which P2 and every later run uses, the crate must be in the demo's graph as well, so that the
// feature's absence is a fact about a dependency that exists. The complete trees are kept in evidence/features.txt. It exits with 1 on a failure.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = path.resolve(argument('--out') ?? path.join(root, 'dess/demo-panel/p0/evidence/features.txt'))
const requireService = process.argv.includes('--require-service-sim')

// The drawing characters are fixed to UTF-8, since a runner whose locale is not UTF-8 may otherwise draw the tree in ASCII, and the reading must not
// depend on which it is.
const tree = pkg => {
  const args = ['tree', '-p', pkg, '--target', 'wasm32-unknown-unknown', '-e', 'features', '--charset', 'utf8']
  const ran = spawnSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 26 })
  if (ran.status !== 0) throw new Error(`cargo ${args.join(' ')} failed:\n${ran.stderr}`)
  return { command: `cargo ${args.join(' ')}`, text: ran.stdout }
}

/** Whether a feature tree says that `service-sim` is built with its `fault-hook` feature. */
export const hasFaultHook = text => /^[│├└─ |`+\\-]*service-sim feature "fault-hook"/m.test(text)
/** Whether `service-sim` is in the tree at all. */
export const hasServiceSim = text => /^[│├└─ |`+\\-]*service-sim feature "/m.test(text) || /service-sim v\d/.test(text)

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

// The reading, on text that is known: a tree that has the feature, one that has the crate without it, and one that has neither.
const withHook =
  'perf-harness-wasm v0.1.0\n├── service-sim feature "fault-hook"\n│   └── service-sim v0.1.0 (/x/service-sim)\n'
const withoutHook =
  'perf-harness-wasm v0.1.0\n├── service-sim feature "default"\n│   └── service-sim v0.1.0 (/x/service-sim)\n'
check(
  hasFaultHook(withHook) && !hasFaultHook(withoutHook) && !hasFaultHook('svg-dom-graph-demo v0.2.0\n'),
  'the reading finds the feature in a tree that has it, and not in one that has the crate without it or lacks the crate'
)
check(
  hasServiceSim(withoutHook) && !hasServiceSim('svg-dom-graph-demo v0.2.0\n'),
  'and it finds the crate where it is, and not where it is not'
)

const demo = tree('svg-dom-graph-demo')
const harness = tree('perf-harness-wasm')
const cargo = spawnSync('cargo', ['--version'], { encoding: 'utf8' }).stdout.trim()

if (!hasFaultHook(harness.text)) {
  const lines = harness.text.split('\n').filter(line => line.includes('service-sim'))
  console.log(
    `the harness graph, which should hold the hook, has these service-sim lines (${lines.length}):\n${lines.slice(0, 20).join('\n')}`
  )
}
check(
  hasFaultHook(harness.text),
  'positive control: the harness build graph holds service-sim "fault-hook", so the same reading does find it when it is there'
)
check(!hasFaultHook(demo.text), 'the demo build graph does not hold service-sim "fault-hook"')
const present = hasServiceSim(demo.text)
if (requireService)
  check(
    present,
    'service-sim is in the demo build graph, so the absence of its fault hook is about a dependency that exists'
  )
else
  console.log(
    `note: service-sim is ${present ? 'in' : 'not yet in'} the demo build graph, ${present ? 'so the absence of its fault hook is a fact about a real dependency' : 'which is expected before P2, and the absence of the hook is then true because the crate is absent. P2 runs this with --require-service-sim'}`
  )

fs.mkdirSync(path.dirname(out), { recursive: true })
fs.writeFileSync(
  out,
  [
    `# P0-D: the feature graphs of the demo build and the harness build`,
    `# ${cargo}`,
    `# measured ${new Date().toISOString()}`,
    `# demo has service-sim: ${present}; demo has fault-hook: ${hasFaultHook(demo.text)}; harness has fault-hook: ${hasFaultHook(harness.text)}`,
    '',
    `## ${demo.command}`,
    demo.text,
    `## ${harness.command}`,
    harness.text
  ].join('\n')
)
console.log(
  failures
    ? `\n${failures} failed`
    : `\nthe demo build does not hold the fault hook, and the check can tell when a build does\nevidence: ${path.relative(root, out)}`
)
process.exit(failures ? 1 : 0)
