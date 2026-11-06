// Attributes a frame to a mutation, from a Chrome trace. This is the correlation rule of design section 3.2.
//
// A mutation is preceded by a user-timing mark, which is a trace event with a time in the trace's clock and, in its arguments,
// the page's own `performance.now()` at the moment it was written. That pair is what ties the trace to the page's clock, so
// no offset between the two has to be assumed.
//
// The frame that carries a mutation is found as follows.
//   1. The main-thread frames are the `ProxyMain::BeginMainFrame` events of the mark's process. Each has the begin-frame id of
//      the compositor's frame, and its time span.
//   2. A frame has a style or layout step if an `UpdateLayoutTree` or `Layout` event of that thread starts inside it.
//   3. The frame is the one that contains the mark if its style or layout step comes after the mark, which is how input that
//      the browser delivers inside a frame reaches the page. Otherwise it is the first frame that begins after the mark and has
//      a style or layout step. A frame that began before the mark and has done its style step already cannot carry it.
//   4. The presentation boundary is the end of the `PipelineReporter` whose `frame_sequence` is the frame's begin-frame id and
//      that ended as `STATE_PRESENTED_ALL`, after the main thread had finished the frame. It is the compositor's own record of
//      when that frame was presented. A part presented earlier, or a frame that was dropped, is not it.
//
// The rule above holds for a change that is made in the task that wrote the mark, or in a frame callback, since the style and
// layout of the change are then done inside the frame that shows it. A change that is followed by a forced layout, such as a
// panel that is built from HTML and measured, has had its style and layout done in the task, and the frame that shows it has
// only a paint. For that the qualifier is widened with `qualify: 'update'`: a frame qualifies if it has a style or layout step
// or repaints something (a `Paint` event). A frame that does neither, such as one that only runs a callback, never qualifies.
// The wider qualifier is for changes that span tasks, and it can take an unrelated repaint for the one it is looking for, so
// it is not the default.
//
// What the figure is, and what it is not. The result is a *trace-attributed presentation latency*: the time from the page's own
// stamp to the compositor's own record of presenting the frame that the rule above attributes to the mark. Three things follow,
// and every figure built on it says them.
//   * It is attribution and not causation. Finding the frame whose lifecycle holds the relevant style or layout work does not
//     prove that every change in that frame was caused by the marked operation. With the default qualifier the mutation is made
//     in the task or frame that wrote the mark, which leaves little room for anything else, and the spike held it to the page's own
//     stamps. With `qualify: 'update'` a frame also qualifies by repainting, and an unrelated repaint can be taken for the one
//     sought, so a figure from that qualifier is the upper bound of a window and not a cause.
//   * It is a presentation by the compositor, and not a photon. Between the compositor's record and light leaving a display there
//     are the display pipeline and the panel's scan-out, which the trace does not see. A headless Chrome presents without any
//     display at all, so its figures say how the page and the library behave, and not how a screen would.
//   * Where the rule cannot attribute a frame, it says so and gives no number.
//
// A sample is unavailable, with its reason, when no such frame is found within the bound, when the mark falls after the style
// step of the frame it is in and a later frame also has one, when the frame was not presented in full, when the mark is missing or
// written twice, or when the trace lost data. Nothing here guesses.

const MAIN_FRAME = 'ProxyMain::BeginMainFrame'
const STYLE_OR_LAYOUT = new Set(['UpdateLayoutTree', 'Layout'])
const PAINT = 'Paint'

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Indexes the events of a trace for attribution.
 *
 * @param events the trace events as Chrome reports them
 * @param options `dataLost` is true if the tracing service said that it dropped events
 */
export const parseTrace = (events, { dataLost = false } = {}) => {
  const marks = new Map()
  const duplicates = new Set()
  const frames = []
  const styleSteps = []
  const paintSteps = []
  const open = new Map()
  const pipelines = []

  for (const event of [...events].sort((a, b) => a.ts - b.ts)) {
    if (event.cat?.includes('blink.user_timing') && event.args?.data?.startTime !== undefined) {
      if (marks.has(event.name)) duplicates.add(event.name)
      marks.set(event.name, {
        name: event.name,
        ts: event.ts,
        startMs: event.args.data.startTime,
        pid: event.pid,
        tid: event.tid
      })
    } else if (event.name === MAIN_FRAME && event.ph === 'X') {
      frames.push({
        id: event.args?.begin_frame_id,
        ts: event.ts,
        end: event.ts + (event.dur ?? 0),
        pid: event.pid,
        tid: event.tid
      })
    } else if (STYLE_OR_LAYOUT.has(event.name) && event.ph === 'X') {
      styleSteps.push({ ts: event.ts, pid: event.pid, tid: event.tid })
    } else if (event.name === PAINT && event.ph === 'X') {
      paintSteps.push({ ts: event.ts, pid: event.pid, tid: event.tid })
    } else if (event.name === 'PipelineReporter') {
      const key = `${event.pid}:${event.id ?? event.id2?.local ?? event.id2?.global}`
      if (event.ph === 'b') open.set(key, event)
      else if (event.ph === 'e' && open.has(key)) {
        const begin = open.get(key)
        open.delete(key)
        const reporter = begin.args?.frame_reporter
        if (reporter)
          pipelines.push({
            sequence: reporter.frame_sequence,
            state: reporter.state,
            begin: begin.ts,
            end: event.ts,
            pid: event.pid
          })
      }
    }
  }
  const within = (steps, frame) =>
    steps
      .filter(step => step.pid === frame.pid && step.tid === frame.tid && step.ts >= frame.ts && step.ts <= frame.end)
      .map(step => step.ts)
  for (const frame of frames) {
    frame.styleAt = within(styleSteps, frame)
    frame.updateAt = [...frame.styleAt, ...within(paintSteps, frame)].sort((a, b) => a - b)
    frame.hasStyleOrLayout = frame.styleAt.length > 0
    frame.hasUpdate = frame.updateAt.length > 0
  }
  frames.sort((a, b) => a.ts - b.ts)
  return { marks, duplicates, frames, pipelines, dataLost }
}

const unavailable = reason => ({ status: 'unavailable', reason })

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * The frame that presented the mutation that the mark `name` precedes.
 *
 * Gives `{ status: 'measured', frame, presentedAt, latencyMs }`, where `presentedAt` is in the trace's clock in microseconds
 * and `latencyMs` is from the mark to the presentation, in the page's clock. Otherwise gives `{ status: 'unavailable',
 * reason }`.
 */
/** The sentence that every figure built on `attribute` carries. */
export const DEFINITION =
  "trace-attributed presentation latency: from the page's own stamp to the compositor's record of presenting the frame that the correlation rule attributes to it. It is not a photon latency, and it does not prove that every change in that frame was caused by the marked operation"

export const attribute = (trace, name, { boundMs = 250, qualify = 'style' } = {}) => {
  if (trace.dataLost) return unavailable('the trace lost data, so a frame may be missing from it')
  if (trace.duplicates.has(name)) return unavailable(`the mark ${name} was written more than once`)
  const mark = trace.marks.get(name)
  if (!mark) return unavailable(`the mark ${name} is not in the trace`)
  const bound = mark.ts + boundMs * 1000

  const mine = trace.frames.filter(frame => frame.pid === mark.pid && frame.tid === mark.tid)
  // A frame that holds the mark with a style step still to come, or the first later frame that has one.
  const containing = mine.find(frame => frame.ts <= mark.ts && mark.ts <= frame.end)
  const steps = frame => (qualify === 'update' ? frame.updateAt : frame.styleAt)
  const candidates = []
  if (containing && steps(containing).some(at => at >= mark.ts)) candidates.push(containing)
  for (const frame of mine) {
    if (frame.ts > mark.ts && frame.ts <= bound && steps(frame).length > 0) candidates.push(frame)
    if (candidates.length > 1) break
  }
  if (containing && !candidates.includes(containing) && candidates.length > 0 && steps(containing).length > 0) {
    return unavailable(
      'the mark fell after the style step of its frame, and a later frame also has one, so they cannot be told apart'
    )
  }
  const frame = candidates[0]
  if (!frame)
    return unavailable(
      `no frame with a ${qualify === 'update' ? 'style, layout or paint' : 'style or layout'} step began within ${boundMs} ms of the mark`
    )

  // A compositor frame is presented in whole or in part. A part that is presented while the main thread is still at work is
  // not the main thread's update, so only a presentation of everything, after the main thread has finished, is this frame's.
  const mine_ = trace.pipelines.filter(p => p.sequence === frame.id && p.pid === frame.pid)
  if (mine_.length === 0) return unavailable(`the compositor has no record of frame ${frame.id} in the trace`)
  const pipeline = mine_.find(p => p.state === 'STATE_PRESENTED_ALL' && p.end >= frame.end)
  if (!pipeline)
    return unavailable(
      `frame ${frame.id} was not presented in full, it ended as ${mine_.map(p => p.state).join(' and ')}`
    )
  if (pipeline.end < mark.ts) return unavailable(`frame ${frame.id} was presented before the mark`)
  return {
    status: 'measured',
    // The qualifier the frame was found with, so that a figure can say how strong its attribution is.
    qualify,
    frame: { id: frame.id, beganAt: frame.ts, endedAt: frame.end },
    presentedAt: pipeline.end,
    latencyMs: (pipeline.end - mark.ts) / 1000,
    // The same instant on the page's clock, which is how a start that the page stamped for itself is compared with it.
    presentedAtPageMs: mark.startMs + (pipeline.end - mark.ts) / 1000
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** Converts an instant of the trace's clock to the page's, using the mark `name` as the pair. */
export const toPageMs = (trace, name, ts) => {
  const mark = trace.marks.get(name)
  return mark ? mark.startMs + (ts - mark.ts) / 1000 : undefined
}
