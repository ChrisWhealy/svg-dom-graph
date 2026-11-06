// Checks the frame attribution of `frames.mjs` against traces that are written out by hand, so that every rule has a case
// that must come out one way and a case that must come out the other. It needs no browser.
//
//   node perf/lib/test-frames.mjs
import { DEFINITION, attribute, parseTrace, toPageMs } from './frames.mjs'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const PID = 1
const TID = 10
// Times are in microseconds of the trace's clock. A mark's page time is its trace time less 1,000,000 us, in ms.
const mark = (name, ts, pid = PID) => ({
  cat: 'blink.user_timing',
  name,
  ph: 'I',
  ts,
  pid,
  tid: TID,
  args: { data: { startTime: (ts - 1_000_000) / 1000 } }
})
const frame = (id, ts, dur, pid = PID) => ({
  cat: 'cc,benchmark',
  name: 'ProxyMain::BeginMainFrame',
  ph: 'X',
  ts,
  dur,
  pid,
  tid: TID,
  args: { begin_frame_id: id }
})
const style = (ts, pid = PID) => ({
  cat: 'blink,devtools.timeline',
  name: 'UpdateLayoutTree',
  ph: 'X',
  ts,
  dur: 20,
  pid,
  tid: TID,
  args: {}
})
const paint = (ts, pid = PID) => ({
  cat: 'blink,devtools.timeline',
  name: 'Paint',
  ph: 'X',
  ts,
  dur: 30,
  pid,
  tid: TID,
  args: {}
})
const pipeline = (id, sequence, begin, end, state = 'STATE_PRESENTED_ALL', slot = '0x1', pid = PID) => [
  {
    cat: 'cc,benchmark',
    name: 'PipelineReporter',
    ph: 'b',
    ts: begin,
    pid,
    tid: 99,
    id: slot,
    args: { frame_reporter: { frame_sequence: sequence, state } }
  },
  { cat: 'cc,benchmark', name: 'PipelineReporter', ph: 'e', ts: end, pid, tid: 99, id: slot, args: {} }
]
const run = (name, events, options) => attribute(parseTrace(events, options), name)

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
{
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.frame.id === 7 && result.latencyMs === 12,
    `a frame that begins after the mark and has a style step carries it, ${result.latencyMs} ms to presentation`
  )
  check(result.presentedAtPageMs === 12, 'the presentation is also given on the page clock, through the mark')
  check(
    toPageMs(parseTrace(events), 'm', 1_005_000) === 5,
    'an instant of the trace converts to the page clock through a mark'
  )
}
{
  // Frame 6 began after the mark but has nothing to restyle. It is not the frame, and frame 7 is.
  const events = [
    mark('m', 1_000_000),
    frame(6, 1_000_100, 100),
    frame(7, 1_016_000, 4_000),
    style(1_017_000),
    ...pipeline(0, 6, 1_000_050, 1_001_000),
    ...pipeline(0, 7, 1_016_000, 1_030_000, 'STATE_PRESENTED_ALL', '0x2')
  ]
  const result = run('m', events)
  check(result.status === 'measured' && result.frame.id === 7, 'a frame with no style or layout step is skipped')
}
{
  // The naive rule, the next presentation after the mark, would take frame 5, which was already in flight.
  const events = [
    mark('m', 1_000_000),
    frame(5, 990_000, 3_000),
    style(991_000),
    ...pipeline(0, 5, 990_000, 1_004_000),
    frame(6, 1_010_000, 4_000),
    style(1_011_000),
    ...pipeline(0, 6, 1_010_000, 1_026_000, 'STATE_PRESENTED_ALL', '0x2')
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.frame.id === 6 && result.latencyMs === 26,
    `a frame already in flight when the mark was written is not taken for it (frame ${result.frame?.id})`
  )
}
{
  // Input that the browser delivers inside a frame: the mark is written inside the frame, before its style step.
  const events = [
    mark('m', 1_002_000),
    frame(8, 1_000_000, 6_000),
    style(1_004_000),
    frame(9, 1_016_000, 4_000),
    style(1_017_000),
    ...pipeline(0, 8, 999_000, 1_010_000),
    ...pipeline(0, 9, 1_016_000, 1_030_000, 'STATE_PRESENTED_ALL', '0x2')
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.frame.id === 8 && result.latencyMs === 8,
    'a mark inside a frame, before its style step, belongs to that frame and not the next'
  )
}
{
  // The same mark after the style step of its frame, with a later frame that also has one: it cannot be told which.
  const events = [
    mark('m', 1_005_000),
    frame(8, 1_000_000, 6_000),
    style(1_004_000),
    frame(9, 1_016_000, 4_000),
    style(1_017_000),
    ...pipeline(0, 8, 999_000, 1_010_000),
    ...pipeline(0, 9, 1_016_000, 1_030_000, 'STATE_PRESENTED_ALL', '0x2')
  ]
  const result = run('m', events)
  check(
    result.status === 'unavailable' && /cannot be told apart/.test(result.reason),
    `a mark after its frame's style step, with a later frame that has one, is ambiguous: ${result.reason}`
  )
}
{
  const result = run('m', [
    mark('m', 1_000_000),
    frame(7, 1_400_000, 1_000),
    style(1_400_500),
    ...pipeline(0, 7, 1_400_000, 1_410_000)
  ])
  check(
    result.status === 'unavailable' && /within 250 ms/.test(result.reason),
    `no frame within the bound is unavailable: ${result.reason}`
  )
}
{
  const result = run('m', [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_001_000, 'STATE_NO_UPDATE_DESIRED')
  ])
  check(
    result.status === 'unavailable' && /not presented in full/.test(result.reason),
    `a frame that was not presented is unavailable: ${result.reason}`
  )
}
{
  // Chrome reports a part of a frame that it presented while the main thread was still at work, and the whole of it later.
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_001_000, 'STATE_PRESENTED_PARTIAL', '0x1'),
    ...pipeline(0, 7, 1_000_050, 1_020_000, 'STATE_PRESENTED_ALL', '0x2')
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.latencyMs === 20,
    `a part that was presented early is not taken for the frame, ${result.latencyMs} ms to the whole`
  )
  const only = run('m', [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_001_000, 'STATE_PRESENTED_PARTIAL')
  ])
  check(only.status === 'unavailable', `a frame of which only a part was presented is unavailable: ${only.reason}`)
  const dropped = run('m', [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_030_000, 'STATE_DROPPED')
  ])
  check(
    dropped.status === 'unavailable' && /STATE_DROPPED/.test(dropped.reason),
    `a dropped frame is unavailable: ${dropped.reason}`
  )
}
{
  const result = run('m', [mark('m', 1_000_000), frame(7, 1_000_100, 5_000), style(1_004_000)])
  check(
    result.status === 'unavailable' && /no record/.test(result.reason),
    `a frame with no compositor record is unavailable: ${result.reason}`
  )
}
{
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const result = run('m', events, { dataLost: true })
  check(
    result.status === 'unavailable' && /lost data/.test(result.reason),
    `a trace that lost data is unavailable: ${result.reason}`
  )
}
{
  const events = [
    mark('m', 1_000_000),
    mark('m', 1_000_500),
    frame(7, 1_001_000, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const result = run('m', events)
  check(
    result.status === 'unavailable' && /more than once/.test(result.reason),
    `a mark that was written twice is unavailable: ${result.reason}`
  )
  check(run('absent', events).status === 'unavailable', 'a mark that is not in the trace is unavailable')
}
{
  // Another process has a frame with the same id and an earlier presentation. It must not be read for this page.
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000),
    frame(7, 1_000_200, 100, 2),
    style(1_000_250, 2),
    ...pipeline(0, 7, 1_000_050, 1_001_000, 'STATE_PRESENTED_ALL', '0x1', 2)
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.latencyMs === 12,
    'the frames and pipelines of another process are not read for this page'
  )
}
{
  // A pipeline identifier is reused. The pairs must be made in order, so that each end meets its own beginning.
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    ...pipeline(0, 3, 900_000, 950_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const result = run('m', events)
  check(
    result.status === 'measured' && result.latencyMs === 12,
    'a reused pipeline identifier pairs each end with its own beginning'
  )
}
{
  // A change whose style and layout were forced in the task that made it: the frame that shows it only paints.
  const events = [
    mark('m', 1_000_000),
    style(1_000_500),
    frame(7, 1_001_000, 4_000),
    paint(1_002_000),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const strict = run('m', events)
  check(
    strict.status === 'unavailable' && /style or layout/.test(strict.reason),
    `a frame that only paints is not a style frame: ${strict.reason}`
  )
  const wide = attribute(parseTrace(events), 'm', { qualify: 'update' })
  check(
    wide.status === 'measured' && wide.frame.id === 7 && wide.latencyMs === 12,
    'with the wider qualifier, the frame that paints carries it'
  )
  const idle = attribute(
    parseTrace([mark('m', 1_000_000), frame(7, 1_001_000, 4_000), ...pipeline(0, 7, 1_000_050, 1_012_000)]),
    'm',
    { qualify: 'update' }
  )
  check(
    idle.status === 'unavailable',
    `a frame that neither restyles nor paints never qualifies, however wide: ${idle.reason}`
  )
}
{
  // A figure says how strongly its frame was attributed, and what it is not.
  const events = [
    mark('m', 1_000_000),
    frame(7, 1_000_100, 5_000),
    style(1_004_000),
    paint(1_004_500),
    ...pipeline(0, 7, 1_000_050, 1_012_000)
  ]
  const strict = attribute(parseTrace(events), 'm')
  const wide = attribute(parseTrace(events), 'm', { qualify: 'update' })
  check(
    strict.qualify === 'style' && wide.qualify === 'update',
    'a measured sample says which qualifier its frame was found with'
  )
  check(
    /not a photon latency/.test(DEFINITION) && /does not prove/.test(DEFINITION),
    'the definition says that it is not photon latency and that attribution is not causation'
  )
}
console.log(
  failures ? `\n${failures} did not behave` : '\nthe attribution rule behaves as designed on every hand-written trace'
)
process.exit(failures ? 1 : 0)
