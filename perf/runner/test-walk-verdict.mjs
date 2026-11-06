// Checks the verdict of a gallery walk on walks written out by hand, so that every outcome has a case, and so that a mismatch
// is shown never to count as a pass. It needs no browser.
//
//   node perf/runner/test-walk-verdict.mjs
import { judgeWalk, unwalked } from './layer-c/walk-verdict.mjs'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const all = { next: true, previous: true, restart: true }
const press = (label, changed = true, phase = label) => ({ label, changed, phase })
// A walk of three steps forward, three back, one more forward to give Restart something to undo, and a restart.
const steps = [
  press('next'),
  press('next'),
  press('next'),
  press('previous'),
  press('previous'),
  press('previous'),
  press('next', true, 'nudge'),
  press('restart')
]
const clean = { start: 'a', forward: ['b', 'c', 'd'], back: ['c', 'b', 'a'], beforeRestart: 'b', afterRestart: 'a' }
const nothing = { start: 'a', forward: [], back: [], beforeRestart: null, afterRestart: null }
const walk = (overrides = {}) => judgeWalk({ found: all, steps, fingerprints: clean, errors: [], ...overrides })
const outcomeOf = (result, name) => result.checks.find(c => c.name === name)?.outcome

{
  const result = walk({ expect: { reversible: true, restart_to_start: true, because: 'its source says so' } })
  check(
    result.verdict === 'pass' && result.checks.every(c => c.outcome === 'pass'),
    'every defined expectation held: pass'
  )
}
{
  const result = walk()
  check(
    result.verdict === 'pass' &&
      outcomeOf(result, 'back_retraces_forward') === 'inconclusive' &&
      outcomeOf(result, 'restart_returns_to_start') === 'inconclusive',
    'with nothing declared, the controls that changed the panel pass and the invariants are inconclusive'
  )
  check(
    /an observation and not a pass/.test(result.checks.find(c => c.name === 'back_retraces_forward').detail),
    'and an undeclared retrace is said to be an observation and not a pass'
  )
}
{
  const result = walk({ fingerprints: { ...clean, back: ['x', 'b', 'a'] } })
  check(
    result.verdict === 'pass' &&
      outcomeOf(result, 'back_retraces_forward') === 'inconclusive' &&
      /an observation and not a failure/.test(result.checks.find(c => c.name === 'back_retraces_forward').detail),
    'an undeclared mismatch is inconclusive, not a failure, and is written down'
  )
}
{
  const result = walk({
    fingerprints: { ...clean, back: ['x', 'b', 'a'] },
    expect: { reversible: true, because: 'selection.rs' }
  })
  check(
    result.verdict === 'fail' && outcomeOf(result, 'back_retraces_forward') === 'fail',
    'a declared reversal that did not happen is a failure, and so is the walk'
  )
}
{
  const result = walk({
    fingerprints: { ...clean, afterRestart: 'z' },
    expect: { restart_to_start: true, because: 'x' }
  })
  check(
    result.verdict === 'fail' && outcomeOf(result, 'restart_returns_to_start') === 'fail',
    'a declared restart that did not return is a failure'
  )
}
{
  const result = walk({
    fingerprints: { ...clean, afterRestart: 'z' },
    expect: { restart_to_start: false, because: 'new random weights' }
  })
  check(
    result.verdict === 'pass' &&
      outcomeOf(result, 'restart_returns_to_start') === 'inconclusive' &&
      /declared not to hold/.test(result.checks.find(c => c.name === 'restart_returns_to_start').detail),
    'a panel that declares it does not restart to its start is not failed for it'
  )
}
{
  const idle = [press('next'), press('next', false), press('previous'), press('restart')]
  const result = judgeWalk({
    found: all,
    steps: idle,
    fingerprints: { start: 'a', forward: ['b', 'c'], back: ['b'], beforeRestart: 'b', afterRestart: 'a' },
    errors: []
  })
  check(
    result.verdict === 'fail' && outcomeOf(result, 'next_changes_panel') === 'fail',
    'an enabled Next that changed nothing is a failure, whatever else passed'
  )
}
{
  const result = walk({ steps: steps.map(step => (step.label === 'restart' ? press('restart', false) : step)) })
  check(
    result.verdict === 'fail' && outcomeOf(result, 'restart_changes_panel') === 'fail',
    'a Restart that changed nothing in a moved panel is a failure'
  )
}
{
  const result = walk({
    fingerprints: { ...clean, beforeRestart: 'a' },
    steps: steps.map(step => (step.label === 'restart' ? press('restart', false) : step))
  })
  check(
    outcomeOf(result, 'restart_changes_panel') === 'inconclusive',
    'a Restart pressed on a panel that was still at its start has nothing to do, and is inconclusive'
  )
}
{
  const result = judgeWalk({
    found: { next: false, previous: false, restart: false },
    steps: [],
    fingerprints: nothing,
    errors: []
  })
  check(
    result.verdict === 'pass' && result.checks.filter(c => c.outcome === 'inconclusive').length === 5,
    'a panel with no controls has every control check inconclusive (the page-error check alone passes)'
  )
  const none = unwalked('no control')
  check(none.verdict === 'inconclusive', 'a panel that was not walked is inconclusive')
}
{
  const only = judgeWalk({
    found: { next: false, previous: false, restart: false },
    steps: [],
    fingerprints: nothing,
    errors: [{ kind: 'exception', text: 'boom' }]
  })
  check(
    only.verdict === 'fail' && outcomeOf(only, 'no_page_errors_during_walk') === 'fail',
    'a page error during the walk is a failure'
  )
}
{
  // The principle: an observed mismatch must not silently count as success. A walk whose only passing check is a control that
  // changed the panel does not turn a declared invariant that failed into a pass.
  const result = walk({
    fingerprints: { ...clean, back: ['x', 'b', 'a'], afterRestart: 'z' },
    expect: { reversible: true, restart_to_start: true, because: 'x' }
  })
  check(
    result.verdict === 'fail' && result.checks.filter(c => c.outcome === 'fail').length === 2,
    'two broken declared invariants are two failures'
  )
}
{
  const short = walk({
    steps: [press('next'), press('next'), press('previous'), press('next', true, 'nudge'), press('restart')],
    expect: { reversible: true, because: 'x' }
  })
  check(
    outcomeOf(short, 'back_retraces_forward') === 'fail',
    'a declared reversal where Previous stopped early is a failure'
  )
}
{
  // Previous past the first step reaches the position before it, which is not compared, so a demo that does not treat it as its
  // unstarted state is not failed for it.
  const result = walk({
    fingerprints: { ...clean, back: ['c', 'b', 'not the start'] },
    expect: { reversible: true, because: 'x' }
  })
  check(
    outcomeOf(result, 'back_retraces_forward') === 'pass',
    'the last Previous, which leaves the first step, is not held to the start'
  )
}
{
  const two = judgeWalk({
    found: all,
    steps: [press('next'), press('previous')],
    fingerprints: { start: 'a', forward: ['b'], back: ['a'], beforeRestart: null, afterRestart: null },
    errors: [],
    expect: { reversible: true, because: 'x' }
  })
  check(
    outcomeOf(two, 'back_retraces_forward') === 'inconclusive',
    'a walk of one step has nothing to retrace, and is inconclusive even when declared'
  )
}
console.log(
  failures ? `\n${failures} did not behave` : '\nevery outcome of a walk is reachable, and a mismatch is never a pass'
)
process.exit(failures ? 1 : 0)
