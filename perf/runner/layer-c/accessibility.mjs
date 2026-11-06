// A bounded check of what Chrome's accessibility tree says about the scene, before and after real input.
//
// The gallery adapter finds controls by their accessible names and the interaction runner checks what the pointer did, and
// neither is a look at the tree itself. This reads Chrome's own (`Accessibility.getFullAXTree`) and holds it to the reference
// model:
//
//   * Roles, names and relationships. A node that has an edge is a `group` whose name is its key followed by one clause for
//     each edge, `Output to X.` or `Input from Y.`, and the clauses are the model's edges as a multiset, whatever their order.
//     The graph view is one `application` whose name carries the zoom. A clause may only name a node that exists.
//   * Bounds. At most `MAX_NODES` nodes are inspected, taken evenly from those that have an edge, and the whole tree may not
//     be larger than a few times the scene, so a tree that grows without limit is a failure.
//   * Focus. After Tab the one focused node that is not the page itself is the graph view, and it is the element the DOM calls
//     active, which is compared by the browser's own node identity and not by name.
//   * Consistency after input. After a drag, a pan and a wheel zoom the roles and names of the inspected nodes are what they
//     were, since moving a node changes no relationship, the graph view's name follows the view's scale, and focus is still
//     the same element in both views of it.
//   * Teardown. With the scene gone, the tree holds no group and no graph view.
//
// Switching the accessibility tree on makes Blink keep it up to date, which costs time on every change. So this is run in a tab
// of its own, after every timed case, and never inside a timing interval.

/** The most nodes inspected for their role, name and relationships. */
export const MAX_NODES = 40
/** The tree may hold this many nodes for each node of the scene, and this many more besides, before it is called unbounded. */
const PER_NODE = 8
const SPARE = 60

const value = property => property?.value
const clauses = name => [...name.matchAll(/(Input from|Output to) ([^.]+)\./g)].map(match => [match[1], match[2]])
const keyOf = name => name.split('.')[0]

/** What the model says each node's name must contain, from the edges of the stream. */
export const expectations = edges => {
  const nodes = new Map()
  const note = key => nodes.get(key) ?? nodes.set(key, { outs: [], ins: [] }).get(key)
  for (const [from, to] of edges) {
    note(from).outs.push(to)
    note(to).ins.push(from)
  }
  return nodes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const readTree = async page => (await page.send('Accessibility.getFullAXTree')).result.nodes

/** The roles and names of the groups, by node key, and the graph view, from a tree. */
const summarise = tree => {
  const groups = new Map()
  const views = []
  for (const node of tree) {
    if (node.ignored) continue
    const role = value(node.role)
    const name = value(node.name)
    if (role === 'group' && name) groups.set(keyOf(name), [...(groups.get(keyOf(name)) ?? []), { role, name }])
    if (role === 'application') views.push({ role, name: name ?? '' })
  }
  return { groups, views, size: tree.length }
}

/** The nodes to inspect: all that have an edge, or an even spread of MAX_NODES of them. */
const sampleOf = keys => {
  if (keys.length <= MAX_NODES) return keys
  const step = keys.length / MAX_NODES
  return Array.from({ length: MAX_NODES }, (_, index) => keys[Math.floor(index * step)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** Holds a summary of the tree to the model, and gives the checks that were made. */
const inspect = (summary, expected, scale, label) => {
  const checks = []
  const check = (ok, detail) => checks.push({ ok, detail: `${label}: ${detail}` })
  const keys = [...expected.keys()].sort((a, b) => a.localeCompare(b, 'en', { numeric: true }))
  const sample = sampleOf(keys)

  check(
    summary.size <= PER_NODE * keys.length + SPARE,
    `the tree holds ${summary.size} nodes, within the bound of ${PER_NODE * keys.length + SPARE}`
  )
  check(summary.views.length === 1, `there is one graph view (${summary.views.length})`)
  const zoom = `Graph view, zoom ${Math.round(scale * 100)}%`
  check(
    summary.views[0]?.name === zoom,
    `the graph view is named "${summary.views[0]?.name}", and the view's scale says "${zoom}"`
  )

  for (const key of sample) {
    const found = summary.groups.get(key) ?? []
    if (found.length !== 1) {
      check(false, `${key} is exposed as ${found.length} groups, and the model has it as one`)
      continue
    }
    const { outs, ins } = expected.get(key)
    const mine = clauses(found[0].name)
    const named = direction =>
      mine
        .filter(([kind]) => kind === direction)
        .map(([, other]) => other)
        .sort()
    const want = list => [...list].sort()
    const same = (a, b) => a.length === b.length && a.every((item, index) => item === b[index])
    check(
      same(named('Output to'), want(outs)) && same(named('Input from'), want(ins)),
      `${key} says "${found[0].name}", and the model has it output to [${want(outs)}] and input from [${want(ins)}]`
    )
  }
  // A relationship has to be with something. A clause that names a node that is not there is a dangling reference.
  const dangling = [...summary.groups.values()]
    .flat()
    .flatMap(group => clauses(group.name))
    .filter(([, other]) => !expected.has(other))
  check(dangling.length === 0, `every relationship names a node that exists (${dangling.length} do not)`)
  return { checks, sample }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The browser's identity of the element the DOM calls active, and of the focused node in the tree that is not the page. */
const focusOf = async page => {
  const tree = await readTree(page)
  const focused = tree.filter(
    node =>
      value(node.role) !== 'RootWebArea' &&
      (node.properties ?? []).some(property => property.name === 'focused' && value(property.value) === true)
  )
  const active = (await page.send('Runtime.evaluate', { expression: 'document.activeElement' })).result.result
  const described = active.objectId
    ? (await page.send('DOM.describeNode', { objectId: active.objectId })).result.node
    : null
  const tag = await page.evaluate(
    'document.activeElement === document.body ? "body" : document.activeElement.localName'
  )
  return { focused, domBackendId: tag === 'body' ? null : described?.backendNodeId, tag }
}

const press = async (page, key, code, keyCode) => {
  await page.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode: keyCode })
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: keyCode })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Runs the check for one case in `tab`, which has the scene of that case built, and gives `{ checks, stats }`.
 *
 * `gestures` are played, with `play`, between the first look and the last.
 */
export const checkAccessibility = async ({ tab, expected, scalePercent, gestures, play, label, fault = null }) => {
  const checks = []
  const add = more => checks.push(...more)
  const scale = scalePercent / 100
  await tab.send('Accessibility.enable')
  await tab.send('DOM.enable')
  if (fault === 'label') await tab.evaluate('perfC.sabotageAccessibility()')

  const before = summarise(await readTree(tab))
  const first = inspect(before, expected, scale, `${label} before input`)
  add(first.checks)

  // Focus: Tab reaches the graph view, and the tree and the DOM agree that it is the element that has focus.
  let landed = null
  for (let attempt = 0; attempt < 4 && landed === null; attempt++) {
    await press(tab, 'Tab', 'Tab', 9)
    const state = await focusOf(tab)
    if (state.tag === 'rect') landed = state
  }
  const agree = state => state.focused.length === 1 && state.focused[0].backendDOMNodeId === state.domBackendId
  checks.push({
    ok: landed !== null && agree(landed) && value(landed.focused[0].role) === 'application',
    detail: `${label}: after Tab the one focused node in the tree is the graph view, and it is the element the DOM calls active (${landed ? `${landed.focused.length} focused, role ${landed.focused[0] && value(landed.focused[0].role)}` : 'Tab never reached the graph view'})`
  })

  // Input, and then the same look again.
  let scaleNow = scale
  for (const gesture of gestures) {
    await play(gesture)
    scaleNow = JSON.parse(await tab.evaluate('perfC.interactive_view()')).scale
  }
  await tab.evaluate('perfC.settle(80)')
  const after = summarise(await readTree(tab))
  add(inspect(after, expected, scaleNow, `${label} after ${gestures.map(g => g.gesture).join(', ')}`).checks)
  const changed = first.sample.filter(
    key => JSON.stringify(before.groups.get(key)) !== JSON.stringify(after.groups.get(key))
  )
  checks.push({
    ok: changed.length === 0,
    detail: `${label}: the roles and names of the ${first.sample.length} inspected nodes are what they were before the input (${changed.length} differ)`
  })
  const late = await focusOf(tab)
  checks.push({
    ok: late.tag === 'body' || agree(late),
    detail: `${label}: after the input, focus is still one element in both the tree and the DOM (${late.tag}, ${late.focused.length} focused in the tree)`
  })

  await tab.evaluate('perfC.interactive_teardown()')
  await tab.evaluate('perfC.settle(40)')
  const gone = summarise(await readTree(tab))
  checks.push({
    ok: gone.views.length === 0 && gone.groups.size === 0,
    detail: `${label}: with the scene torn down the tree holds no graph view and no group (${gone.views.length}, ${gone.groups.size})`
  })
  return {
    checks,
    stats: {
      ax_nodes: Math.max(before.size, after.size),
      nodes_inspected: first.sample.length,
      nodes_with_an_edge: expected.size
    }
  }
}
