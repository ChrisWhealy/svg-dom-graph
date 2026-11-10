// Writes the contracts of the simulator's fault points into perf/fault-contracts.json, with the lines that make each promise found by what
// they say and not typed in, so that a source that moves is found again and the contract is never left citing a line that has gone.
//
//   node dess/s4/write-contracts.mjs [--check]
//
// perf/lib/test-contracts.mjs checks that every cited range exists. This checks that it is the range that makes the promise, by finding
// the anchor text in the file, and says so if the text is not there. `--check` writes nothing and exits 1 if the file is not what this
// would write, so CI notices a contract that no longer matches the code.
import fs from 'node:fs'
import path from 'node:path'

const root = path.resolve(import.meta.dirname, '../..')
const file = path.join(root, 'perf/fault-contracts.json')
const check = process.argv.includes('--check')

const CONTROLLER = 'service-sim/src/controller/mod.rs'
const SURFACE = 'service-sim/src/surface.rs'
const HOST = 'service-sim/src/host.rs'
const SCHEDULER = 'service-sim/src/scheduler.rs'
const SETTER = 'src/scene/node/data/values.rs'
const DATA_NODE = 'src/scene/node/data/construct.rs'

// A source is the file, the text at which its range begins, and how many lines it runs.
const at = (source, anchor, lines = 1) => ({ source, anchor, lines })

const POINTS = [
  {
    id: 'service_data_node_construction',
    operation: 'Surface::build, which draws the scene, the overlay and the reserved data node',
    contract: 'nothing_committed',
    says: "A failure at any DOM call while the surface is built returns the error and makes no surface, so no Surface exists to draw on. The scene's own add_data_node draws a node as a transaction, so a failure there leaves the scene as it was. The caller removes the host and builds again on a fresh one. A failure in the first draw of the controller that is built on it is a draw like any other, and belongs to the overlay and the setter.",
    retry: 'Building again on a fresh host.',
    sources: [
      at(SURFACE, 'A failure at any DOM call returns the error', 3),
      at(DATA_NODE, 'Drawing a node is a transaction', 4)
    ]
  },
  {
    id: 'service_data_setter',
    operation: 'Scene::set_data_values on the reserved data node, which every draw makes',
    contract: 'partly_committed',
    differs: ['content', 'accessibility'],
    says: "A failure part way through the cells leaves some cells rewritten and the node's values as they were, and marks the node. The next call, whatever values it asks for, writes every cell, changed or not, and clears the mark when it has written them all. Until finding F1 of the S0 audit was closed, a call that asked for the values the node still held changed nothing and left the node torn.",
    retry:
      "One call to Scene::set_data_values with the last committed values. The controller's repair is that call, as part of the full redraw.",
    sources: [at(SETTER, 'Also returns a wrapped', 8)]
  },
  {
    id: 'service_overlay_write',
    operation: 'The draw of a slice, which writes the fields of the overlay and then the data node',
    contract: 'partly_committed',
    differs: ['content', 'accessibility'],
    says: 'A write that fails leaves the page partly written. No new slice is committed, the cursor and the model are as they were, playback is stopped, the page is marked dirty and one failure is reported. The controller then makes one immediate full redraw of the committed slice, and if that fails the page stays dirty and stopped.',
    retry: 'Repair, which draws the committed slice again in full, and then the slice again.',
    sources: [
      at(SURFACE, 'A draw that fails part way leaves the page partly written', 3),
      at(CONTROLLER, 'One immediate, bounded repair', 2)
    ]
  },
  {
    id: 'service_control_aria',
    operation: "The writes of the Play control's ARIA state and label, which Play and Pause make",
    contract: 'partly_committed',
    differs: ['accessibility'],
    says: 'A write of the control that fails leaves it half written and the page says nothing: the cursor is where it was, and playback is as the controller says. The host remembers that the control is stale, and a full draw writes its content and ARIA again from what the controller last said. A repair never starts or stops playback.',
    retry: 'Repair, which writes the control again.',
    sources: [at(HOST, 'A write that fails leaves the control half written', 3), at(HOST, 'A full draw is a repair', 3)]
  },
  {
    id: 'service_timer_request',
    operation: 'The request for the timer that the next slice of playback waits for',
    contract: 'nothing_committed',
    says: 'If the page refuses a timer the callback does not exist, so nothing is committed. The last committed display stays, playback stops, and one failure is reported. The next timer is asked for only after a slice was committed.',
    retry: 'Play, which asks for the timer again.',
    sources: [
      at(CONTROLLER, 'The last committed display stays', 2),
      at(SCHEDULER, 'Returns `Err` if the task could not be scheduled', 1)
    ]
  },
  {
    id: 'service_frame_request',
    operation: 'The request for the animation frame that a seek replays at',
    contract: 'nothing_committed',
    says: 'If the page refuses a frame the seek is not started and nothing is committed. The cursor stays, the target is kept as the retry, one failure is reported, and no synchronous replay is made in its place.',
    retry: 'Retry, which asks for the frame again.',
    sources: [
      at(CONTROLLER, 'The cursor stays, and a person can ask again', 2),
      at(SCHEDULER, 'Returns `Err` if the frame could not be asked for', 1)
    ]
  },
  {
    id: 'service_gallery_hidden',
    operation: 'A gallery panel that holds the host is hidden, with the hint that the page gives and without it',
    contract: 'nothing_committed',
    says: 'Hiding stops playback, takes back what was scheduled, and moves the epoch and the seek token, so a callback that was already on its way is stale. It is not promised that no callback runs while the panel is hidden. It is promised that one that does is inert: it commits nothing and writes nothing. Showing the panel again repairs a dirty page and never resumes playback.',
    retry: 'Play, which a person presses after the panel is shown.',
    sources: [
      at(CONTROLLER, 'Three counters keep a stale callback inert', 6),
      at(HOST, 'The hint that a gallery panel was shown or hidden', 4)
    ]
  },
  {
    id: 'service_detached_before_draw',
    operation: 'A draw that is asked for while the host is out of the document',
    contract: 'nothing_committed',
    says: 'A detached element accepts writes without complaint, so the host is checked and not discovered: the surface refuses a detached host before any write, and the controller does not draw. Nothing is written and nothing is committed. On return the committed slice is drawn again in full, and the cursor does not advance.',
    retry: 'Presence, which is told when the host returns, and then the slice again.',
    sources: [
      at(SURFACE, 'Fails with [`DrawError::Detached`] if the host is not in the document', 3),
      at(CONTROLLER, 'The commit of design section 6.1', 1)
    ]
  },
  {
    id: 'service_detached_during_draw',
    operation: 'A draw whose host leaves the document after any of its writes',
    contract: 'partly_committed',
    differs: ['content'],
    says: 'A write that went through is no evidence that the page is attached, so the guard is asked again after the draw. A host that left in the middle of it leaves the page partly written, and the slice is not committed: the cursor and the model are as they were, playback is stopped, the page is marked dirty and the host is announced as unavailable once.',
    retry: 'Presence, told when the host returns, which draws the committed slice again in full.',
    sources: [
      at(CONTROLLER, 'that went through is no evidence that the page is attached', 2),
      at(SURFACE, 'A draw that fails part way leaves the page partly written', 3)
    ]
  },
  {
    id: 'service_teardown_late_tick',
    operation: 'A timer or frame callback that runs after the controller was disposed',
    contract: 'nothing_committed',
    says: 'Disposing marks the controller first, then takes back what it scheduled, so a callback that was already on its way finds the controller disposed or its token moved and does nothing: no slice is committed, no DOM is written and no notice is changed. It holds only a weak reference, so nothing is kept alive, and no closure or listener is left.',
    retry: 'None: a disposed controller is never resumed.',
    sources: [at(CONTROLLER, 'Ends the controller for good', 2), at(CONTROLLER, 'fn on_tick', 8)]
  }
]

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const linesOf = new Map()
const lines = source => {
  if (!linesOf.has(source)) linesOf.set(source, fs.readFileSync(path.join(root, source), 'utf8').split('\n'))
  return linesOf.get(source)
}
let missing = 0
const entries = POINTS.map(point => {
  const sources = point.sources.map(({ source, anchor, lines: count }) => {
    const found = lines(source).findIndex(line => line.includes(anchor))
    if (found < 0) {
      console.error(`${point.id}: "${anchor}" is not in ${source}`)
      missing += 1
      return `${source}:1-1`
    }
    return `${source}:${found + 1}-${found + count}`
  })
  const { sources: _anchors, ...rest } = point
  return { id: rest.id, target: 'service_simulation', ...rest, documented: 'code', sources }
})
if (missing) process.exit(2)

// The file's own layout: a short array on one line, a long one on one line for each item.
const render = (entry, indent) => {
  const pad = ' '.repeat(indent)
  const field = (key, value) => {
    if (Array.isArray(value)) {
      const inline = `${pad}  "${key}": ${JSON.stringify(value)}`
      return inline.length <= 120
        ? inline
        : `${pad}  "${key}": [\n${value.map(v => `${pad}    ${JSON.stringify(v)}`).join(',\n')}\n${pad}  ]`
    }
    return `${pad}  "${key}": ${JSON.stringify(value)}`
  }
  const order = ['id', 'target', 'operation', 'contract', 'documented', 'says', 'differs', 'retry', 'sources']
  return `${pad}{\n${order
    .filter(key => key in entry)
    .map(key => field(key, entry[key]))
    .join(',\n')}\n${pad}}`
}

const current = fs.readFileSync(file, 'utf8')
const data = JSON.parse(current)
const kept = data.points.filter(p => p.target !== 'service_simulation')
// The graph entries keep their text exactly, and the service entries are written after them.
const cut = current.lastIndexOf('\n  ]\n}')
const graphText = (() => {
  const first = current.indexOf('"points": [') + '"points": ['.length
  const end = current.lastIndexOf('\n  ]')
  const body = current.slice(first, end)
  // Everything up to the first service entry is the graph's.
  const service = body.indexOf('"target": "service_simulation"')
  if (service < 0) return body
  const start = body.lastIndexOf('    {', service)
  return body.slice(0, start).replace(/,\s*$/, '')
})()
void cut
void kept
const next =
  current.slice(0, current.indexOf('"points": [') + '"points": ['.length) +
  graphText +
  ',\n' +
  entries.map(entry => render(entry, 4)).join(',\n') +
  '\n  ]\n}\n'
JSON.parse(next)
if (check) {
  if (next !== current) {
    console.error(
      'perf/fault-contracts.json is not what dess/s4/write-contracts.mjs writes: run it, and look at the difference'
    )
    process.exit(1)
  }
  console.log('the service contracts are what the code says, and their sources are found')
} else {
  fs.writeFileSync(file, next)
  console.log(`wrote ${entries.length} service contracts, each with the lines that make its promise`)
}
