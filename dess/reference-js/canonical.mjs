// The canonical byte encoding of a state, and the other hashes, as the contract (dess/s1/docs/contract.md) fixes them.
import crypto from 'node:crypto'

export const sha256 = data => crypto.createHash('sha256').update(data).digest('hex')

/** A byte writer. Integers are little-endian and unsigned. */
export class Writer {
  constructor() {
    this.bytes = []
  }
  u8(v) {
    this.bytes.push(Number(v) & 0xff)
    return this
  }
  u32(v) {
    let x = Number(v)
    for (let i = 0; i < 4; i++) {
      this.bytes.push(x & 0xff)
      x = Math.floor(x / 256)
    }
    return this
  }
  u64(v) {
    let x = BigInt(v)
    for (let i = 0; i < 8; i++) {
      this.bytes.push(Number(x & 0xffn))
      x >>= 8n
    }
    return this
  }
  string(s) {
    const encoded = Buffer.from(s, 'utf8')
    this.u32(encoded.length)
    for (const b of encoded) this.bytes.push(b)
    return this
  }
  u64List(list) {
    this.u32(list.length)
    for (const v of list) this.u64(v)
    return this
  }
  digest() {
    return sha256(Buffer.from(this.bytes))
  }
}

/** The canonical JSON of a value: keys sorted at every level, no spaces. */
export const canonicalJson = value => {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  if (value !== null && typeof value === 'object')
    return `{${Object.keys(value)
      .sort()
      .map(k => `${JSON.stringify(k)}:${canonicalJson(value[k])}`)
      .join(',')}}`
  return JSON.stringify(value)
}
