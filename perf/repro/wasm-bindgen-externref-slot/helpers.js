// Three imports that do what a DOM call does when it is made to fail: they throw. They return an object, return nothing, and
// return a number. With `fail` false they return normally, which is the control.
export function returns_object(fail) {
  if (fail) throw new Error('failed')
  return {}
}

export function returns_nothing(fail) {
  if (fail) throw new Error('failed')
}

export function returns_number(fail) {
  if (fail) throw new Error('failed')
  return 1
}
