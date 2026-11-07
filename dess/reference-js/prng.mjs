// SplitMix64, and the derivation of a named stream from a seed. Written for the oracle from the contract, in BigInt.
const M = (1n << 64n) - 1n
const GAMMA = 0x9e3779b97f4a7c15n

export class Stream {
  constructor(state) {
    this.state = BigInt(state) & M
  }
  next() {
    this.state = (this.state + GAMMA) & M
    let z = this.state
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & M
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & M
    return z ^ (z >> 31n)
  }
  /** 0..n-1 without modulo bias: a draw in the short last interval of the 64-bit range is thrown away. */
  below(n) {
    const bound = BigInt(n)
    const limit = M - (M % bound)
    for (;;) {
      const draw = this.next()
      if (draw < limit) return Number(draw % bound)
    }
  }
  between(lo, hi) {
    return lo + this.below(hi - lo + 1)
  }
}

/** The stream `n` of `seed`: mix the stream number into the seed, then seed a generator with the first output of that. */
export const stream = (seed, n) => {
  const mixed = BigInt(seed) ^ ((BigInt(n) * 0xd6e8feb86659fd93n) & M)
  return new Stream(new Stream(mixed).next())
}
