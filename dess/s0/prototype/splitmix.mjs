// SplitMix64 with the harness's stream derivation, in BigInt so that it is exact. It is the same algorithm as
// `perf-harness/src/prng/mod.rs`, and `test-prototype.mjs` checks it against that file's published vectors.
const MASK = (1n << 64n) - 1n

export class SplitMix64 {
  constructor(seed) {
    this.state = BigInt(seed) & MASK
  }

  next() {
    this.state = (this.state + 0x9e3779b97f4a7c15n) & MASK
    let z = this.state
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK
    return z ^ (z >> 31n)
  }

  /** A number in 0..n, without modulo bias. */
  below(n) {
    const bound = BigInt(n)
    const limit = MASK - (MASK % bound)
    for (;;) {
      const draw = this.next()
      if (draw < limit) return Number(draw % bound)
    }
  }

  /** A number in lo..=hi. */
  between(lo, hi) {
    return lo + this.below(hi - lo + 1)
  }
}

/** A generator for one named stream of a seed, so that unrelated choices do not share a sequence. */
export const derive = (seed, stream) =>
  new SplitMix64(new SplitMix64(BigInt(seed) ^ ((BigInt(stream) * 0xd6e8feb86659fd93n) & MASK)).next())
