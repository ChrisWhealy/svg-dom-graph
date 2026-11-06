// Judges the walk of a gallery panel: Next, Previous and Restart pressed in turn, with the panel's fingerprint taken between.
//
// A walk gives one of three outcomes, and the point of keeping them apart is that a mismatch is never counted as a pass.
//
//   * pass: the control exists, its expected effect was seen, and the invariants that are defined for the panel hold;
//   * inconclusive: a control is absent, or the panel's own navigation has no defined expectation to hold it to, or it is
//     declared not to have one. What was seen is still written down, but it is not called success;
//   * fail: a defined expectation was broken.
//
// What is defined for every panel is small: an enabled Next or Previous changes the panel, an enabled Restart changes a panel
// that has been moved, and the page throws nothing during the walk. Whether Previous undoes Next, and whether Restart goes back
// to the state the panel was first shown in, differ from demo to demo, so they are held only where the panel declares them in
// `perf/gallery-overrides.json`, with the place in its source that says so. Previous undoing Next means that from each step it
// lands on the state that Next left from. A demo may declare that it does not (Dijkstra's
// Restart draws new random weights), and that is inconclusive and not a failure.

const PASS = 'pass'
const FAIL = 'fail'
const INCONCLUSIVE = 'inconclusive'

const check = (name, outcome, detail) => ({ name, outcome, detail })

/** The outcome of a check that every enabled press of a control changed the panel. */
const changes = (name, control, found, presses) => {
  if (!found) return check(name, INCONCLUSIVE, `there is no ${control} control by accessible name`)
  if (presses.length === 0)
    return check(name, INCONCLUSIVE, `${control} was disabled whenever it was looked for, so it was never pressed`)
  const idle = presses.filter(press => !press.changed).length
  return idle === 0
    ? check(name, PASS, `${presses.length} presses of ${control}, and each changed the panel`)
    : check(name, FAIL, `${idle} of ${presses.length} presses of an enabled ${control} changed nothing`)
}

/** The outcome of an invariant that holds only if the panel declared it. */
const declared = (name, expectation, because, holds, observed) => {
  if (expectation === true) {
    return holds === null
      ? check(name, INCONCLUSIVE, 'declared, and the walk did not reach the state it is judged at')
      : holds
        ? check(name, PASS, `declared (${because}), and it held`)
        : check(name, FAIL, `declared (${because}), and it did not hold: ${observed}`)
  }
  if (expectation === false) return check(name, INCONCLUSIVE, `declared not to hold (${because}); ${observed}`)
  return check(name, INCONCLUSIVE, `no expectation is defined for this panel; ${observed}`)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Judges a walk.
 *
 * `found` says which controls the panel has, `steps` is every press with whether it changed the panel (a Next pressed only to
 * give Restart something to undo has the phase `nudge`), and `fingerprints` are the panel's fingerprint at the `start`, after
 * each Next of the walk forward (`forward`), after each Previous that followed (`back`), `beforeRestart` and `afterRestart`
 * (each null if the walk did not reach it), `errors` are what the page threw, `expect` is what the panel declares, and
 * `differences` are readable accounts of where states differ, which are put in the detail of a mismatch.
 */
export const judgeWalk = ({ found, steps, fingerprints, errors, expect = {}, differences = {} }) => {
  const presses = label => steps.filter(step => step.label === label)
  // The Next that is pressed once more before Restart, so that Restart has something to undo, is not part of the walk forward.
  const forward = presses('next').filter(step => step.phase !== 'nudge')
  const back = presses('previous')
  const checks = [
    changes('next_changes_panel', 'Next', found.next, presses('next')),
    changes('previous_changes_panel', 'Previous', found.previous, back)
  ]
  const because = expect.because ?? 'no reason given'

  // Whether Previous undoes Next: from each step, Previous has to land on the state that Next left from. Stepping back past the
  // first step reaches the position before it, which a demo may or may not treat as its unstarted state, so that one is not judged.
  const comparable = Math.max(0, Math.min(forward.length, back.length) - 1)
  const retraced = []
  for (let index = 0; index < comparable; index++) {
    // The (index + 1)th Previous leaves step forward.length - index - 1, and Next left that step with forward[step - 1].
    const step = forward.length - index - 1
    retraced.push({ step, matched: fingerprints.back[index] === fingerprints.forward[step - 1] })
  }
  const broken = retraced.filter(r => !r.matched)
  const holds = forward.length < 2 || back.length < 2 ? null : broken.length === 0
  const observed =
    comparable === 0
      ? 'the walk had fewer than two steps to retrace'
      : broken.length === 0
        ? `Previous landed on the state Next had left from at all ${comparable} steps it could be compared`
        : `Previous missed the state Next had left from at ${broken.length} of ${comparable} steps (${broken.map(r => r.step).join(', ')})${differences.back ? `: ${differences.back}` : ''}`
  const reversal =
    expect.reversible === true && forward.length > 0 && back.length < forward.length
      ? check(
          'back_retraces_forward',
          FAIL,
          `declared (${because}), and Previous stopped after ${back.length} of ${forward.length} steps`
        )
      : declared('back_retraces_forward', expect.reversible, because, holds, observed)
  if (reversal.outcome === INCONCLUSIVE && expect.reversible === undefined && holds !== null) {
    reversal.detail += holds ? ' (an observation and not a pass)' : ' (an observation and not a failure)'
  }
  checks.push(reversal)

  // Restart acts on a panel that has been moved off its start, or there is nothing for it to do.
  const restarts = presses('restart')
  const moved = fingerprints.beforeRestart !== null && fingerprints.beforeRestart !== fingerprints.start
  checks.push(
    !found.restart
      ? check('restart_changes_panel', INCONCLUSIVE, 'there is no Restart control by accessible name')
      : restarts.length === 0
        ? check(
            'restart_changes_panel',
            INCONCLUSIVE,
            'Restart was disabled, or the panel could not be moved off its start for it to act on'
          )
        : !moved
          ? check(
              'restart_changes_panel',
              INCONCLUSIVE,
              'the panel was not away from its start when Restart was pressed'
            )
          : restarts[0].changed
            ? check('restart_changes_panel', PASS, 'Restart changed a panel that had been moved')
            : check(
                'restart_changes_panel',
                FAIL,
                `Restart changed nothing in a panel that had been moved${fingerprints.beforeRestart === fingerprints.afterRestart ? ' (the fingerprints before and after are the same)' : ''}`
              )
  )
  const restartedTo =
    restarts.length > 0 && fingerprints.afterRestart !== null ? fingerprints.afterRestart === fingerprints.start : null
  const restart = declared(
    'restart_returns_to_start',
    expect.restart_to_start,
    because,
    restartedTo,
    `the panel is not what it was at the start${differences.restart ? ` (${differences.restart})` : ''}`
  )
  if (restart.outcome === INCONCLUSIVE && expect.restart_to_start === undefined && restartedTo !== null) {
    restart.detail += restartedTo
      ? ' (it did return to the start, which is an observation and not a pass)'
      : ' (it did not return to the start, which is an observation and not a failure)'
  }
  checks.push(restart)

  checks.push(
    errors.length === 0
      ? check('no_page_errors_during_walk', PASS, 'the page threw nothing and logged no error')
      : check('no_page_errors_during_walk', FAIL, `${errors.length} errors: ${errors[0].text.slice(0, 120)}`)
  )
  const verdict = checks.some(c => c.outcome === FAIL)
    ? FAIL
    : checks.some(c => c.outcome === PASS)
      ? PASS
      : INCONCLUSIVE
  return { verdict, checks }
}

/** The verdict of a panel that was not walked. */
export const unwalked = reason => ({ verdict: INCONCLUSIVE, checks: [check('walk', INCONCLUSIVE, reason)] })
