// Records a Chrome trace of the categories that frame attribution reads, around a function.
import { parseTrace } from './frames.mjs'

/** The categories the correlation rule needs: the harness's marks, the main thread's work and the compositor's pipeline. */
export const CATEGORIES = [
  'blink.user_timing',
  'devtools.timeline',
  'disabled-by-default-devtools.timeline',
  'disabled-by-default-devtools.timeline.frame',
  'benchmark',
  'cc',
  'viz'
]

/**
 * Runs `work` with a trace recording, and gives back what it returned with the parsed trace.
 *
 * `bufferKb` is for the test of a trace that loses data, which asks for a buffer that is too small.
 */
export const traced = async (page, work, { bufferKb } = {}) => {
  const events = []
  const stop = page.on('Tracing.dataCollected', params => events.push(...params.value))
  const complete = new Promise(resolve => {
    const off = page.on('Tracing.tracingComplete', params => {
      off()
      resolve(params)
    })
  })
  await page.send('Tracing.start', {
    transferMode: 'ReportEvents',
    traceConfig: {
      recordMode: bufferKb ? 'recordUntilFull' : 'recordContinuously',
      includedCategories: CATEGORIES,
      ...(bufferKb ? { traceBufferSizeInKb: bufferKb } : {})
    }
  })
  const result = await work()
  // Frames that are still on their way to the screen when the work is over must reach the trace before it is closed.
  await new Promise(resolve => setTimeout(resolve, 150))
  await page.send('Tracing.end')
  const done = await complete
  stop()
  return {
    result,
    trace: parseTrace(events, { dataLost: done.dataLossOccurred === true }),
    events,
    dataLossOccurred: done.dataLossOccurred === true
  }
}
