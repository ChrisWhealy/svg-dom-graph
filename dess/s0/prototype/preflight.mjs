// The activity and stability test of a W6 smoke trace (design revision 6, section 8.3), and the windows it is applied to.
//
// The bounds were written in the design before any trace was measured, and they are frozen here. Changing one is a revision of
// the design with a reason that does not depend on which seeds passed.
export const WINDOW = { priming: 256, warmup: 40, measured: 240 } // slices 1-256, 257-296, 297-536
export const MIN_SLICES = WINDOW.priming + WINDOW.warmup + WINDOW.measured // 536
export const MIN_POPS = 10_000
export const BOUNDS = {
  occupancyMean: [0.25, 0.9],
  waitingNonemptyMin: 0.5,
  popsPerSlice: [1.5, 32],
  busyMin: 0.5,
  halfOccupancyDifferenceMax: 0.2,
  halfPopsDifferenceMax: 0.25
}

const mean = values => values.reduce((a, b) => a + b, 0) / values.length

/** `slices`: the first 536 or more slices, each `{ index, pops, waiting, busy }`. `capacity`: the queue capacity. */
export const evaluate = (slices, capacity, totalPops) => {
  const reasons = []
  if (slices.length < MIN_SLICES) reasons.push(`only ${slices.length} nonempty slices, and ${MIN_SLICES} are needed`)
  if (totalPops < MIN_POPS) reasons.push(`only ${totalPops} pops, and ${MIN_POPS} are needed`)
  const from = WINDOW.priming + WINDOW.warmup
  const window = slices.slice(from, from + WINDOW.measured)
  if (window.length < WINDOW.measured) return { qualified: false, reasons, metrics: null }

  const occupancy = window.map(s => s.waiting / capacity)
  const pops = window.map(s => s.pops)
  const half = WINDOW.measured / 2
  const metrics = {
    occupancyMean: mean(occupancy),
    waitingNonemptyFraction: mean(window.map(s => (s.waiting > 0 ? 1 : 0))),
    popsPerSlice: mean(pops),
    busyFraction: mean(window.map(s => (s.busy >= 1 ? 1 : 0))),
    halfOccupancyDifference: Math.abs(mean(occupancy.slice(0, half)) - mean(occupancy.slice(half))),
    halfPopsDifference: Math.abs(mean(pops.slice(0, half)) - mean(pops.slice(half))) / mean(pops)
  }
  const b = BOUNDS
  const fail = text => reasons.push(text)
  if (metrics.occupancyMean < b.occupancyMean[0] || metrics.occupancyMean > b.occupancyMean[1])
    fail(`mean occupancy ${metrics.occupancyMean.toFixed(3)} is outside ${b.occupancyMean[0]} to ${b.occupancyMean[1]}`)
  if (metrics.waitingNonemptyFraction < b.waitingNonemptyMin)
    fail(
      `the queue was nonempty in ${(metrics.waitingNonemptyFraction * 100).toFixed(0)}% of slices, and ${b.waitingNonemptyMin * 100}% are needed`
    )
  if (metrics.popsPerSlice < b.popsPerSlice[0] || metrics.popsPerSlice > b.popsPerSlice[1])
    fail(`${metrics.popsPerSlice.toFixed(2)} pops a slice is outside ${b.popsPerSlice[0]} to ${b.popsPerSlice[1]}`)
  if (metrics.busyFraction < b.busyMin)
    fail(
      `a server was busy in ${(metrics.busyFraction * 100).toFixed(0)}% of slices, and ${b.busyMin * 100}% are needed`
    )
  if (metrics.halfOccupancyDifference > b.halfOccupancyDifferenceMax)
    fail(
      `the halves differ by ${metrics.halfOccupancyDifference.toFixed(3)} in occupancy, and ${b.halfOccupancyDifferenceMax} is the most`
    )
  if (metrics.halfPopsDifference > b.halfPopsDifferenceMax)
    fail(
      `the halves differ by ${(metrics.halfPopsDifference * 100).toFixed(1)}% in pops a slice, and ${b.halfPopsDifferenceMax * 100}% is the most`
    )
  return { qualified: reasons.length === 0, reasons, metrics }
}
