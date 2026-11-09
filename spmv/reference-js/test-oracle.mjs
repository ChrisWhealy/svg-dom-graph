// The oracle against what can be known without the Rust core: the generator's reference outputs, a matrix written out by hand, and
// the cases of `spmv/cases.json`, whose expectations were derived by hand.
//
//   node spmv/reference-js/test-oracle.mjs
import fs from 'node:fs'
import { Stream, stream } from './prng.mjs'
import { label, matrixHash, multiply, outcome, panelSpec, validate, vectorHash } from './oracle.mjs'

let failures = 0
const check = (ok, what) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`)
  if (!ok) failures++
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)

// The reference outputs of SplitMix64 for the seed 1234567, as perf-harness and the simulator fix them.
{
  const g = new Stream(1234567)
  const want = [
    6457827717110365317n,
    3203168211198807973n,
    9817491932198370423n,
    4593380528125082431n,
    16408922859458223821n
  ]
  check(
    want.every(w => g.next() === w),
    'SplitMix64 gives the reference sequence'
  )
  check(
    stream(5, 1).next() === stream(5, 1).next() && stream(5, 1).next() !== stream(5, 2).next(),
    'streams are repeatable and distinct'
  )
}

// A matrix written out by hand:   r0 [2 0 -1 0]   r1 [0 0 0 0]   r2 [3 4 0 5]
{
  const m = [
    [
      [0, 2],
      [2, -1]
    ],
    [],
    [
      [0, 3],
      [1, 4],
      [3, 5]
    ]
  ]
  check(same(multiply(m, [1, 10, 100, -1]), [-98, 0, 38]), 'the product of a matrix written by hand')
  check(
    matrixHash(m, 4) === '2e2f3949569a9b726f35e4946ffa0d2f62a50ec221006d27ac0018891ab6067a',
    'the hash of that matrix is the one the Rust core pins'
  )
  check(
    vectorHash([1, 10, 100, -1]) === 'f45761e606f61b883f930b91bf546bdb6bb76dbcd532be3f6132a99279061a14',
    'the hash of a vector is the one the Rust core pins'
  )
  check(label('x', 7, 6) === 'x7----' && label('y', 12345, 3) === 'y12345', 'labels are padded and never cut')
}

// Every panel seed is a valid specification.
{
  let bad = 0
  for (let seed = 0; seed < 2000; seed++) {
    try {
      validate(panelSpec(seed))
    } catch {
      bad++
    }
  }
  check(bad === 0, 'every panel seed is a valid specification')
}

// The cases whose expectations were derived by hand.
{
  const cases = JSON.parse(fs.readFileSync(new URL('../cases.json', import.meta.url), 'utf8'))
  let wrong = 0
  let checked = 0
  for (const [name, c] of Object.entries(cases)) {
    if (!c.expect) continue
    checked++
    const got = outcome(c.spec, c.budget)
    const fields = ['outcome', 'code', 'message'].filter(k => c.expect[k] !== undefined)
    const ok = fields.every(k => got[k] === c.expect[k]) && (!c.expect.stats || same(got.stats, c.expect.stats))
    if (!ok) {
      wrong++
      console.log(`     ${name}: ${JSON.stringify(got).slice(0, 200)}, and ${JSON.stringify(c.expect)} was expected`)
    }
  }
  check(
    wrong === 0 && checked >= 20,
    `the ${checked} cases with an expectation give the outcomes that were derived by hand`
  )
}

if (failures > 0) {
  console.log(`\n${failures} checks failed`)
  process.exit(1)
}
console.log('\nthe oracle gives the known answers')
