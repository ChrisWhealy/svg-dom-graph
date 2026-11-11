// The P0 acceptance run: every P0 script, from a clean tree, so that the evidence names the commit that it measured.
//
//   node dess/demo-panel/p0/run-all.mjs [--allow-dirty]
//
// The evidence files record the git commit and whether the tree had uncommitted changes when they were made. Evidence from a dirty tree does not say
// what was measured, so this refuses to run on one, unless `--allow-dirty` is given for trying it out. On a clean tree it runs the staging probe, the
// injector self-test (with the graph recovery self-test that shares the injector), the injector's mutation check and the feature-isolation check with
// `--require-service-sim` left off until the demo depends on `service-sim` (P2), and stops at the first failure. The evidence is written under
// dess/demo-panel/p0/evidence/, and the files that change are then for a person to review and commit.
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const allowDirty = process.argv.includes('--allow-dirty')
const status = spawnSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).stdout.trim()
if (status && !allowDirty) {
  console.error('the tree has uncommitted changes, so the evidence would not say what was measured:')
  console.error(
    status
      .split('\n')
      .slice(0, 10)
      .map(line => `  ${line}`)
      .join('\n')
  )
  console.error('Commit or stash them and run again, or use --allow-dirty to try it out.')
  process.exit(2)
}
const commit = spawnSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: root, encoding: 'utf8' }).stdout.trim()
console.log(`P0 acceptance from commit ${commit}${status ? ' (with uncommitted changes)' : ', a clean tree'}\n`)

const STEPS = [
  ['staging probe', 'dess/demo-panel/p0/staging.mjs'],
  ['injector self-test', 'dess/demo-panel/p0/injector.mjs'],
  ['injector mutation check', 'dess/demo-panel/p0/injector-mutation-check.mjs'],
  ['feature isolation', 'dess/demo-panel/p0/features.mjs']
]
for (const [name, script] of STEPS) {
  console.log(`=== ${name}`)
  const ran = spawnSync('node', [path.join(root, script)], { cwd: root, stdio: 'inherit' })
  if (ran.status !== 0) {
    console.error(`\n${name} failed, so P0 is not accepted from this commit`)
    process.exit(1)
  }
}
console.log(`\nall P0 gates pass from commit ${commit}. Review dess/demo-panel/p0/evidence/ and commit it.`)
