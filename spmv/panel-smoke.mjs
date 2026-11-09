// Drives the sparse-matrix demo in headless Chrome the way a person would, and checks what it says and what it leaves behind.
//
//   cargo run -p demo-server -- --build-only      # stages the gallery in target/demo-stage
//   node spmv/panel-smoke.mjs [--stage DIR] [--screenshot PREFIX]
//
// The gallery's panel only describes the demo and opens it in a window, so the checks are on that window: that every list and every
// button is in a `<foreignObject>` of its one `<svg>`, that the graph pans and zooms, that a click on a node lights its edges and writes
// out the arithmetic, that a run stops when asked and when the window is hidden and that no step follows, that a size or a label over the
// budget is refused with its reason and leaves the scene empty, that ten rebuilds leave the page as it was, and that nothing was
// thrown. Nothing here is a timing gate.
import fs from 'node:fs'
import path from 'node:path'
import { launch, serve } from '../perf/lib/chrome-session.mjs'
import { pause } from '../perf/lib/input.mjs'

const root = path.resolve(import.meta.dirname, '..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const stage = path.resolve(argument('--stage') ?? path.join(root, 'target/demo-stage'))
const screenshot = argument('--screenshot')

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const { server, port } = await serve(stage)
const chrome = await launch()
const thrown = []
chrome.on('Runtime.exceptionThrown', e => thrown.push(e.exceptionDetails?.exception?.description ?? e.exceptionDetails?.text))
chrome.on('Runtime.consoleAPICalled', e => {
  if (e.type === 'error') thrown.push(`console.error: ${(e.args ?? []).map(a => a.value ?? a.description).join(' ')}`)
})
await chrome.send('Runtime.enable')
await chrome.send('Emulation.setDeviceMetricsOverride', { width: 1500, height: 950, deviceScaleFactor: 1, mobile: false })
try {
  const text = id => chrome.evaluate(`document.getElementById('${id}')?.textContent ?? null`)
  const click = id => chrome.evaluate(`document.getElementById('${id}').click()`)
  const choose = (id, value) => chrome.evaluate(`(document.getElementById('${id}').value = '${value}')`)
  const waitFor = async (predicate, ms = 30000) => {
    for (let waited = 0; waited < ms; waited += 50) {
      if (await predicate()) return true
      await pause(50)
    }
    return false
  }

  // - - - The gallery's panel only describes the demo and opens it.
  await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/#panel-sparse-matrix` })
  // The panel is wired when the gallery has built it, which is when its source frame is added.
  await waitFor(() => chrome.evaluate(`!!document.querySelector('#panel-sparse-matrix .source')`), 20000)
  check((await chrome.evaluate(`document.querySelectorAll('#panel-sparse-matrix select').length`)) === 0, 'the gallery panel holds no list')
  check(
    (await chrome.evaluate(`document.querySelectorAll('#panel-sparse-matrix button').length`)) === 1,
    'the gallery panel holds one button, which opens the demo'
  )
  await chrome.evaluate(`window.__opened = null; window.open = (...a) => { window.__opened = a; return { focus() {} } }`)
  await click('spmv-open')
  const opened = await chrome.evaluate(`window.__opened`)
  check(opened?.[0] === 'sparse-matrix-window.html' && /popup/.test(opened?.[2] ?? ''), `the button opens the window (${JSON.stringify(opened)})`)

  // - - - The window.
  await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/sparse-matrix-window.html` })
  check(await waitFor(async () => (await text('spmv-verdict'))?.startsWith('Accepted')), 'the first scenario is built when the window opens')
  const first = await text('spmv-verdict')
  check(/Accepted: 16 nodes and \d+ edges/.test(first), `the first scenario is small enough to read (${first})`)

  // Every list and every button is in a <foreignObject> of the one <svg>, and none is on the page round it.
  const strays = await chrome.evaluate(
    `[...document.querySelectorAll('select, button, input')].filter(e => !e.closest('svg#spmv-host foreignObject')).map(e => e.id || e.tagName)`
  )
  check(strays.length === 0, `every list and button is in a <foreignObject> of the window's <svg> (${strays.length} outside${strays[0] ? `: ${strays}` : ''})`)
  const inside = await chrome.evaluate(`document.querySelectorAll('svg#spmv-host foreignObject select, svg#spmv-host foreignObject button').length`)
  check(inside >= 16, `the controls are there (${inside}: seven lists and nine buttons)`)
  check((await chrome.evaluate(`document.querySelector('.spmv-scene-svg')?.closest('svg#spmv-host') !== null`)), 'the scene is in the same <svg>')

  // The controls look as those of the Petri net demo do, whose buttons are the library's SVG buttons: a white fill, an outline of 1.5
  // pixels in slate with a corner of 6, and text of 14 pixels in near black. The lists are the browser's own, with their font inherited.
  const style = (id, props) =>
    chrome.evaluate(`(() => { const s = getComputedStyle(document.getElementById('${id}')); return ${JSON.stringify(props)}.map(p => s.getPropertyValue(p)) })()`)
  const look = ['background-color', 'box-shadow', 'border-top-left-radius', 'color', 'font-size']
  const resting = ['rgb(255, 255, 255)', 'rgb(100, 116, 139) 0px 0px 0px 1.5px inset', '6px', 'rgb(26, 26, 26)', '14px']
  for (const id of ['spmv-build', 'spmv-step', 'spmv-zoom-in', 'spmv-teardown']) {
    const got = await style(id, look)
    check(got.join('|') === resting.join('|'), `${id} is drawn as the library draws a button (${got.join(', ')})`)
  }
  const pane = await style('spmv-size', ['font-size', 'color'])
  check(pane[0] === '14px', `a list is in the size that the Petri net's lists are (${pane[0]})`)
  const paneColour = await chrome.evaluate(`getComputedStyle(document.querySelector('.spmv-controls-pane')).backgroundColor`)
  check(paneColour === 'rgb(255, 255, 238)', `the controls are on the colour of the demos' canvas (${paneColour})`)
  // - - - The multiplication is drawn as the operations that do it, for the matrix that a person can read.
  const mouse = (type, x, y) => chrome.send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 })
  // Keys, for the checks that use no mouse: Tab moves the focus, text is typed, and Enter commits it.
  const key = (name, code, vk) => chrome.send('Input.dispatchKeyEvent', { type: name === 'Enter' ? 'keyDown' : 'rawKeyDown', key: name, code, windowsVirtualKeyCode: vk, text: name === 'Enter' ? '\r' : undefined })
    .then(() => chrome.send('Input.dispatchKeyEvent', { type: 'keyUp', key: name, code, windowsVirtualKeyCode: vk }))
  const typeNode = async word => {
    await chrome.evaluate(`(() => { const i = document.getElementById('spmv-goto'); i.focus(); i.select() })()`)
    await chrome.send('Input.insertText', { text: word })
    await key('Enter', 'Enter', 13)
    await pause(300)
  }
  const worked = () =>
    chrome.evaluate(`(() => {
      const svg = document.querySelector('.spmv-scene-svg')
      const groups = [...svg.querySelectorAll(':scope > g.svg-dom-graph-content > g')]
      const cellsOf = (g, n) => [...g.querySelectorAll('rect')].slice(-n).map(r => r.getBoundingClientRect())
      const valuesOf = (g, n) => [...g.querySelectorAll('text')].slice(-2 * n).filter((_, i) => i % 2 === 1).map(t => t.textContent)
      const x = cellsOf(groups[1], 8), y = valuesOf(groups[2], 8)
      const result = svg.querySelector('.spmv-term-result text:last-of-type')?.textContent
      const heading = svg.querySelector('.spmv-heading')?.textContent
      return {
        a: svg.querySelectorAll('rect.spmv-a-cell').length, zero: svg.querySelectorAll('rect.spmv-a-cell.spmv-zero').length,
        lit: svg.querySelectorAll('rect.spmv-a-cell.spmv-on').length,
        products: svg.querySelectorAll('.spmv-term-product').length, sums: svg.querySelectorAll('.spmv-term-sum').length,
        xColumn: x.every(r => Math.abs(r.left - x[0].left) < 1) && x.every((r, i) => i === 0 || r.top > x[i - 1].top),
        heading, result, y, changed: svg.querySelectorAll('rect.spmv-changed').length
      }
    })()`)
  let w = await worked()
  check(w.a === 64 && w.zero > 0 && w.zero < 64, `A is a grid of its 64 numbers, with its zeros marked (${w.a} cells, ${w.zero} zeros)`)
  check(w.xColumn, 'the vector x is a single column and not a grid')
  check(w.products === w.lit && w.sums === Math.max(0, w.lit - 1), `a multiplication for each number of the row and an addition between them (${w.lit} lit, ${w.products} multiplications, ${w.sums} additions)`)
  const rowOf = heading => Number(/^Row (\d+) of A times x, added up, makes y\1:$/.exec(heading ?? '')?.[1] ?? -1)
  check(rowOf(w.heading) >= 0 && w.result === w.y[rowOf(w.heading)], `the sum ends at the entry of y for its row (${w.heading}: ${w.result} against ${w.y[rowOf(w.heading)]})`)
  // A click on a number of another row of A shows that row.
  const centreOfA = row => chrome.evaluate(`(() => { const r = document.querySelectorAll('rect.spmv-a-cell[data-spmv-node="y${row}"]')[3].getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2] })()`)
  const [cx, cy] = await centreOfA(5)
  await mouse('mousePressed', cx, cy)
  await mouse('mouseReleased', cx, cy)
  await pause(300)
  w = await worked()
  check(rowOf(w.heading) === 5 && w.result === w.y[5] && (await text('spmv-selected')).startsWith('y5 is made from row 5'), `a click on a row of A shows the operations of that row (${w.heading})`)
  const viewBefore = await chrome.evaluate(`document.querySelector('.spmv-scene-svg > g.svg-dom-graph-content').getAttribute('transform')`)
  await click('spmv-step')
  w = await worked()
  check(w.result === w.y[5] && w.changed > 0, `after a step the sum still ends at y5, and what changed is outlined (${w.changed} entries)`)
  check(/the input changed \(x\d+ -?\d+ → -?\d+\)/.test(await text('spmv-story')), `a step is explained in words (${await text('spmv-story')})`)
  check(viewBefore === (await chrome.evaluate(`document.querySelector('.spmv-scene-svg > g.svg-dom-graph-content').getAttribute('transform')`)), 'showing another row keeps the view')
  // A node can be reached from the keyboard alone: Tab from the list lands on the box, and a row typed there is chosen.
  await chrome.evaluate(`document.getElementById('spmv-pick').focus()`)
  await key('Tab', 'Tab', 9)
  check((await chrome.evaluate(`document.activeElement?.id`)) === 'spmv-goto', 'Tab from the list of nodes moves the focus to the box for typing one')
  await typeNode('Y3')
  check((await text('spmv-selected')).startsWith('y3 is made from row 3'), 'typing a row selects it, whatever the case')
  w = await worked()
  check(rowOf(w.heading) === 3 && w.result === w.y[3], `the operations of the typed row are drawn (${w.heading})`)
  await typeNode('x3')
  check(/one row at a time/.test(await text('spmv-error')) && rowOf((await worked()).heading) === 3, `a column is declined in this view, with the reason (${await text('spmv-error')})`)
  await typeNode('y99')
  check(/There is no y99: the rows are y0 to y7\./.test(await text('spmv-error')), `a node that is not there is declined with the range (${await text('spmv-error')})`)

  // Steps go in order, so that what comes next can be told, and a run is slow enough to follow.
  check((await chrome.evaluate(`document.getElementById('spmv-order').value`)) === 'inorder', 'a matrix that is worked steps in order by default')
  check(/x0 then x1/.test(await text('spmv-task')) && /Steps go in order/.test(await text('spmv-story') + await text('spmv-task')), 'the window says that steps go in order')
  const changedX = async () => Number(/the input changed \(x(\d+) /.exec(await text('spmv-story'))?.[1] ?? -1)
  const order = [await changedX()]
  for (let k = 0; k < 2; k++) {
    await click('spmv-step')
    order.push(await changedX())
  }
  check(order.every((j, k) => k === 0 || j === (order[k - 1] + 1) % 8), `each step changes the next entry of x (${order})`)
  check(/so the next step changes x\d+/.test(await text('spmv-story')), 'the story says which entry comes next')
  const bars = await chrome.evaluate(`({ bars: document.querySelectorAll('.spmv-bar').length, scale: document.querySelector('.spmv-bars-scale')?.textContent, fed: document.querySelectorAll('.spmv-a-cell.spmv-fed').length })`)
  check(bars.bars === 8 && /^a full bar is \d+$/.test(bars.scale) && bars.fed > 0, `the output is drawn as bars, and the numbers of A that the last step fed are marked (${JSON.stringify(bars)})`)
  // The speed of a run is chosen with a list: slow takes few steps, fast takes many.
  const runFor = async (speed, ms) => {
    await choose('spmv-speed', speed)
    const before = Number((await text('spmv-n-step')).match(/^Step (\d+)/)[1])
    await click('spmv-run')
    await pause(ms)
    await click('spmv-run')
    return Number((await text('spmv-n-step')).match(/^Step (\d+)/)[1]) - before
  }
  const slow = await runFor('3000', 4000)
  const fast = await runFor('500', 3200)
  check(slow >= 1 && slow <= 2 && fast >= 4 && fast <= 7 && fast > slow, `the run speed list sets the pace: ${slow} steps in 4 s when slow and ${fast} in 3.2 s when fast`)
  check((await chrome.evaluate(`document.getElementById('spmv-speed').value`)) === '500', 'the list says what was chosen')

  // The page owns the callback of every frame it asks for, so a frame that is cancelled leaves none behind. The window counts the
  // callbacks that are alive, and many starts and stops, a rebuild in the middle of a run, and a window that is hidden leave none.
  const alive = () => chrome.evaluate(`Number(document.getElementById('spmv-root').getAttribute('data-live-frame-callbacks') ?? -1)`)
  await choose('spmv-speed', '0')
  let most = 0
  for (let k = 0; k < 40; k++) {
    await click('spmv-run')
    await pause(k % 3 === 0 ? 40 : 5)
    most = Math.max(most, await alive())
    await click('spmv-run')
  }
  check(most >= 1 && most <= 2, `a run holds at most two frame callbacks, the one that is waiting and the one that ran (${most})`)
  check((await alive()) === 0, 'after 40 starts and stops no frame callback is left behind')
  await click('spmv-run')
  await pause(100)
  await click('spmv-build')
  await pause(300)
  check((await alive()) === 0 && (await chrome.evaluate(`document.getElementById('spmv-run').textContent`)) === 'Run', 'a rebuild in the middle of a run leaves no frame callback behind')
  await choose('spmv-speed', '500')

  // - - - The graph that a larger matrix is drawn as: the rest of the window is checked on 20 by 20.
  await choose('spmv-size', '40')
  await click('spmv-build')
  check(await waitFor(async () => (await text('spmv-verdict'))?.startsWith('Accepted: 40 nodes')), 'a graph of 40 entities is built')
  // The graph is small enough to read: the nodes are big enough on the screen for their text.
  const box = await chrome.evaluate(`(() => { const r = document.querySelector('[data-spmv-node="x0"] rect').getBoundingClientRect(); return [r.width, r.height] })()`)
  check(box[0] >= 44 && box[1] >= 18, `a node is big enough to read on the screen (${box.map(v => v.toFixed(0)).join(' by ')} px)`)

  // The three boxes say what they are.
  const captionText = klass => chrome.evaluate(`[...document.querySelectorAll('.spmv-scene-svg .spmv-caption-${klass}')].map(t => t.textContent).join(' ')`)
  check(/input vector/.test(await captionText('x')), `the first data node is explained (${await captionText('x')})`)
  check(/output/.test(await captionText('y')), `the second data node is explained (${await captionText('y')})`)
  check(/A: the matrix/.test(await captionText('pattern')), `the matrix is explained (${await captionText('pattern')})`)

  // The task is stated, and selecting x0 shows how its value goes into the sums, each of which ends at the y beside its node.
  const overview = await text('spmv-task')
  check(/^What this shows:/m.test(overview) && /^How to read it:/m.test(overview) && /^What a step does:/m.test(overview) && /^Why it matters:/m.test(overview), 'the overview says what this shows, how to read it, what a step does and why it matters')
  check(/Step order/.test(overview), 'the overview says where the order of the steps is chosen')
  await choose('spmv-pick', 'x0')
  await chrome.evaluate(`document.getElementById('spmv-pick').dispatchEvent(new Event('change', { bubbles: true }))`)
  const how = await text('spmv-selected')
  const lines = how.split('\n').filter(l => l.startsWith('row '))
  const yValues = await chrome.evaluate(`[...document.querySelectorAll('.spmv-node-value-y')].map(t => t.textContent)`)
  check(lines.length > 0 && lines.every(l => { const [, i, total] = /a term of y(\d+) = .* = (−?\d+|-\d+)$/.exec(l) ?? []; return total !== undefined && total.replace('−', '-') === yValues[Number(i)] }),
    `selecting x0 shows each sum it is a term of, and each total is the y beside its node (${lines.length} rows)`)
  await choose('spmv-pick', '')
  await chrome.evaluate(`document.getElementById('spmv-pick').dispatchEvent(new Event('change', { bubbles: true }))`)

  // A step says in words what it did, and that the connector numbers are the fixed matrix.
  await click('spmv-step')
  const story = await text('spmv-story')
  check(/the input changed \(x\d+ -?\d+ → -?\d+\)/.test(story) && /A never changes, only x does/.test(story), `a step is explained in words (${story})`)
  check(/of the matrix A is/.test(await captionText('channel')), 'the number on a connector is explained in the scene')

  // - - - Panning and zooming.
  const view = () => chrome.evaluate(`document.querySelector('.spmv-scene-svg > g.svg-dom-graph-content').getAttribute('transform')`)
  const scaleOf = v => Number(/scale\(([^)]+)\)/.exec(v ?? '')?.[1] ?? 1)
  const start = scaleOf(await view())
  await click('spmv-zoom-in')
  const zoomedIn = scaleOf(await view())
  check(zoomedIn > start, `Zoom in zooms in (${start} to ${zoomedIn})`)
  await click('spmv-zoom-out')
  await click('spmv-zoom-out')
  check(scaleOf(await view()) < start, 'Zoom out zooms out')
  await click('spmv-zoom-reset')
  check(scaleOf(await view()) === 1, 'Reset view puts the view back')
  const before = await view()
  await chrome.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: 80, y: 330, button: 'left', clickCount: 1 })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 130, y: 300, button: 'left' })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 180, y: 270, button: 'left' })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 180, y: 270, button: 'left', clickCount: 1 })
  await pause(100)
  check((await view()) !== before, `dragging the background pans (${before} to ${await view()})`)
  await click('spmv-zoom-reset')
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 750, y: 400, deltaX: 0, deltaY: -100, modifiers: 2 })
  await pause(100)
  check(scaleOf(await view()) > 1, 'a wheel turned with Ctrl held zooms')
  await click('spmv-zoom-reset')

  // - - - Every node can be reached, and the background says that it can be dragged, however dense the edges are.
  await choose('spmv-size', '1000')
  await click('spmv-build')
  check(await waitFor(async () => (await text('spmv-verdict'))?.startsWith('Accepted: 1,000 nodes'), 20000), 'a graph of 1,000 entities is built')
  const reach = await chrome.evaluate(`(() => {
    const nodes = [...document.querySelectorAll('[data-spmv-node]')]
    const blocked = []
    for (const g of nodes) {
      const r = g.querySelector('rect').getBoundingClientRect()
      const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)
      if (top?.closest('[data-spmv-node]') !== g) blocked.push(g.dataset.spmvNode + ':' + top?.tagName)
    }
    return { nodes: nodes.length, blocked: blocked.length, first: blocked.slice(0, 3) }
  })()`)
  check(reach.nodes === 1000 && reach.blocked === 0, `every one of the ${reach.nodes} nodes of a dense graph is under the pointer at its centre (${reach.blocked} covered${reach.first[0] ? `, such as ${reach.first}` : ''})`)
  const edgeCursor = await chrome.evaluate(`(() => {
    // The middle of each of 200 edges, where the connector is, in the channel between the two sides.
    const paths = [...document.querySelectorAll('.spmv-scene-svg > g.svg-dom-graph-content > path.spmv-edge')].filter((_, i) => i % 7 === 0).slice(0, 200)
    const seen = {}
    for (const p of paths) {
      const m = p.getScreenCTM(); const at = p.getPointAtLength(p.getTotalLength() / 2)
      const x = m.a * at.x + m.c * at.y + m.e, y = m.b * at.x + m.d * at.y + m.f
      const e = document.elementFromPoint(x, y)
      const kind = e?.closest('[data-spmv-node]') ? 'node' : e?.tagName === 'path' ? 'a connector' : getComputedStyle(e).cursor
      seen[kind] = (seen[kind] ?? 0) + 1
    }
    return seen
  })()`)
  check(!('a connector' in edgeCursor), `the pointer goes through the connectors to what is under them (${JSON.stringify(edgeCursor)})`)
  check(edgeCursor.grab > 0 && Object.keys(edgeCursor).every(k => ['grab', 'node'].includes(k)), 'over the background, wherever an edge passes, the cursor is the hand that drags it')
  // The experiment that found it: a real click on node 250, which a dense graph covered with edges, and a real drag that begins on an edge.
  const centre = tag =>
    chrome.evaluate(`(() => { const r = document.querySelector('[data-spmv-node="${tag}"] rect').getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2] })()`)
  const [nx, ny] = await centre('x250')
  await chrome.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: nx, y: ny, button: 'left', clickCount: 1 })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: nx, y: ny, button: 'left', clickCount: 1 })
  await pause(150)
  check((await text('spmv-selected')).startsWith('x250 = '), `a mouse click on column 250 selects it (${(await text('spmv-selected')).slice(0, 60)})`)
  const [rx, ry] = await centre('y400')
  await chrome.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: rx, y: ry, button: 'left', clickCount: 1 })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: rx, y: ry, button: 'left', clickCount: 1 })
  await pause(150)
  check((await text('spmv-selected')).includes('y400 = '), 'a mouse click on row 400 selects it')
  const onEdge = await chrome.evaluate(`(() => {
    const p = document.querySelector('.spmv-scene-svg > g.svg-dom-graph-content > path.spmv-edge.spmv-w9')
    const m = p.getScreenCTM(); const at = p.getPointAtLength(p.getTotalLength() / 2)
    return [m.a * at.x + m.c * at.y + m.e, m.b * at.x + m.d * at.y + m.f]
  })()`)
  const panBefore = await view()
  await chrome.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: onEdge[0], y: onEdge[1], button: 'left', clickCount: 1 })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: onEdge[0] + 40, y: onEdge[1] + 25, button: 'left' })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: onEdge[0] + 80, y: onEdge[1] + 50, button: 'left' })
  await chrome.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: onEdge[0] + 80, y: onEdge[1] + 50, button: 'left', clickCount: 1 })
  await pause(100)
  check((await view()) !== panBefore, 'a drag that begins on top of an edge pans the scene')
  // A node beyond the first 100 that the list holds is reached from the keyboard, by typing it.
  const inList = await chrome.evaluate(`[...document.getElementById('spmv-pick').options].map(o => o.value)`)
  check(inList.includes('x99') && !inList.includes('x250') && !inList.includes('y400'), 'the list holds only the first 100 of each, so x250 and y400 are not in it')
  await typeNode('x250')
  check((await text('spmv-selected')).startsWith('x250 = '), `typing x250 selects a column that the list does not hold (${(await text('spmv-selected')).slice(0, 40)})`)
  check((await chrome.evaluate(`document.querySelectorAll('[data-spmv-node="x250"].spmv-picked').length`)) === 1, 'the node that was typed is the one that is picked in the scene')
  await typeNode('y400')
  check((await text('spmv-selected')).includes('y400 = '), 'typing y400 selects a row that the list does not hold')
  await typeNode('x5000')
  check(/There is no x5000: the columns are x0 to x499\./.test(await text('spmv-error')) && (await text('spmv-selected')).includes('y400 = '), `a node that is not there is declined with the range, and the selection stays (${await text('spmv-error')})`)
  await typeNode('banana')
  check(/Type a node as x or y/.test(await text('spmv-error')), 'text that is not a node is declined with what to type')
  await click('spmv-zoom-reset')
  await choose('spmv-size', '40')
  await click('spmv-build')

  // - - - A node is selected by a click, and says what it computes.
  const lit = () => chrome.evaluate(`document.querySelectorAll('.spmv-scene-svg path.spmv-on').length`)
  // The two data nodes are the two groups before the pattern square, whatever labels the edges have.
  const cells = () =>
    chrome.evaluate(
      `(() => { const spy = document.querySelector('.spmv-scene-svg g.spmv-aside'); const y = spy.previousElementSibling; const x = y.previousElementSibling; return [x, y].map(g => [...g.querySelectorAll('text')].map(t => t.textContent.trim())) })()`
    )
  check((await lit()) === 0, 'nothing is lit before a node is selected')
  await chrome.evaluate(`document.querySelector('[data-spmv-node="y2"]').dispatchEvent(new MouseEvent('click', { bubbles: true }))`)
  const sentence = (await text('spmv-selected')).split('\n').pop()
  check(/^y2 = .+ = .+ = -?\d+$/.test(sentence.replace(/−/g, '-')), `a click on a row writes out its arithmetic (${sentence})`)
  const degree = (sentence.match(/x\d+/g) ?? []).length
  check((await lit()) === degree && degree > 0, `a click lights exactly the row's ${degree} edges (${await lit()} lit)`)
  const yCells = (await cells())[1]
  check(yCells[2] === sentence.split('= ').pop().replace('−', '-'), `the arithmetic ends at the figure in the y data node (${yCells[2]})`)
  await chrome.evaluate(`document.querySelector('[data-spmv-node="x0"]').dispatchEvent(new MouseEvent('click', { bubbles: true }))`)
  const columnSentence = await text('spmv-selected')
  check(/^x0 = -?\d+\. It is multiplied by a number of A in each row/.test(columnSentence.replace(/−/g, '-')), `a click on a column says which rows it feeds (${columnSentence})`)
  await choose('spmv-pick', 'y5')
  await chrome.evaluate(`document.getElementById('spmv-pick').dispatchEvent(new Event('change', { bubbles: true }))`)
  check((await text('spmv-selected')).split('\n').pop().startsWith('y5 = '), 'choosing a node from the list selects it too')
  await choose('spmv-pick', '')
  await chrome.evaluate(`document.getElementById('spmv-pick').dispatchEvent(new Event('change', { bubbles: true }))`)
  check((await lit()) === 0, 'choosing none puts everything out')

  // - - - A step, a run, and a stop.
  const stepsDone = async () => Number((await text('spmv-n-step'))?.match(/^Step (\d+)/)?.[1] ?? 0)
  await choose('spmv-pick', 'y2')
  await chrome.evaluate(`document.getElementById('spmv-pick').dispatchEvent(new Event('change', { bubbles: true }))`)
  await click('spmv-step')
  check((await text('spmv-n-step')).startsWith('Step 1:'), 'a step is reported')
  const after = (await text('spmv-selected')).split('\n').pop()
  check(after.startsWith('y2 = ') && after.split('= ').pop() === (await cells())[1][2].replace('-', '−'), 'what a selected node computes follows the step, and still ends at the data node')
  const shown = await chrome.evaluate(`({ cells: document.querySelectorAll('.spmv-matrix-cell').length, lit: document.querySelectorAll('.spmv-matrix-cell.spmv-on').length, hot: document.querySelectorAll('rect.spmv-hot').length })`)
  check(shown.cells > 0 && shown.lit > 0 && shown.hot > 0, `the matrix is drawn in full, and the selected node lights its cells and the entries of x and y it uses (${JSON.stringify(shown)})`)
  const elementsBefore = await chrome.evaluate(`document.querySelectorAll('svg#spmv-host *').length`)
  await click('spmv-run')
  check((await chrome.evaluate(`document.getElementById('spmv-run').getAttribute('aria-pressed')`)) === 'true', 'Run says it is pressed')
  await waitFor(async () => (await stepsDone()) > 20, 5000)
  const pressedLook = await style('spmv-run', ['background-color', 'box-shadow'])
  check(pressedLook.join('|') === 'rgb(204, 251, 241)|rgb(15, 118, 110) 0px 0px 0px 1.5px inset', `a pressed button is teal as the library's is (${pressedLook.join(', ')})`)
  check((await stepsDone()) > 20, `a run steps once a frame (${await stepsDone()} steps)`)
  check((await chrome.evaluate(`document.querySelectorAll('svg#spmv-host *').length`)) === elementsBefore, 'a run adds no element')
  await click('spmv-run')
  await pause(100)
  const stopped = await stepsDone()
  check((await style('spmv-run', ['background-color']))[0] === 'rgb(255, 255, 255)', 'a button that is not pressed is white again')
  await pause(400)
  check((await stepsDone()) === stopped, `no step follows a stop (${stopped} steps, and none after)`)

  // A run stops by itself when the window is hidden, and no step follows. The window is really hidden, by bringing another tab to the
  // front, because a hidden page is given no animation frames, and a frame cannot be what notices that it is hidden. Nor does the run
  // go on when the window comes back.
  await click('spmv-run')
  await waitFor(async () => (await stepsDone()) > stopped + 5, 5000)
  const other = await chrome.newPage('about:blank')
  await other.send('Page.bringToFront')
  check(await waitFor(async () => (await chrome.evaluate('document.visibilityState')) === 'hidden', 5000), 'the window is hidden by another tab in front of it')
  await pause(300)
  const hiddenAt = await stepsDone()
  await pause(600)
  check((await stepsDone()) === hiddenAt, `a run stops when the window is hidden, and no step follows (${hiddenAt} steps)`)
  check((await chrome.evaluate(`document.getElementById('spmv-run').getAttribute('aria-pressed')`)) === 'false', 'Run says it is stopped')
  await chrome.send('Page.bringToFront')
  check(await waitFor(async () => (await chrome.evaluate('document.visibilityState')) === 'visible', 5000), 'the window is visible again')
  await pause(800)
  check((await stepsDone()) === hiddenAt, 'a run does not start again when the window comes back')
  check((await alive()) === 0, 'a window that was hidden in the middle of a run leaves no frame callback behind')
  await other.closeTab()

  // - - - Refusals: reported apart from a drawing, with the reason, and a graph that was there is left as it was.
  const sceneSvgs = () => chrome.evaluate(`document.querySelectorAll('.spmv-scene-svg').length`)
  const marker = () =>
    chrome.evaluate(`(() => { const s = document.querySelector('.spmv-scene-svg'); return s ? s.id + ':' + s.querySelectorAll('rect').length + ':' + s.querySelector('g.svg-dom-graph-content').getAttribute('transform') : null })()`)
  for (const [control, value, wanted] of [
    ['spmv-size', '20000', 'nodes 20000 is over the budget of 10000'],
    ['spmv-labels', '1024', 'label length 1024 is over the budget of 256']
  ]) {
    await choose('spmv-size', '16')
    await choose('spmv-labels', '1')
    await click('spmv-build')
    check(await waitFor(async () => (await text('spmv-verdict'))?.startsWith('Accepted: 16 nodes'), 10000), `${control} ${value}: a graph is there before the refusal`)
    await click('spmv-step')
    const had = { marker: await marker(), plan: await text('spmv-n-plan'), step: await text('spmv-n-step'), task: await text('spmv-task') }
    await choose(control, value)
    await click('spmv-build')
    const verdict = await text('spmv-verdict')
    check(verdict.startsWith('Refused before anything was built') && verdict.includes(wanted), `${control} ${value} is refused with its reason (${verdict})`)
    check(verdict.endsWith('The graph you had is unchanged.'), `${control} ${value}: the refusal says that the graph is unchanged`)
    check((await sceneSvgs()) === 1 && (await marker()) === had.marker, `${control} ${value}: the graph that was there is the same one, with the same view`)
    check((await text('spmv-n-plan')) === had.plan && (await text('spmv-n-step')) === had.step && (await text('spmv-task')) === had.task, `${control} ${value}: the readouts of that graph are as they were`)
    const stepBefore = await text('spmv-n-step')
    await click('spmv-step')
    check((await text('spmv-n-step')) !== stepBefore && /^Step 2:/.test(await text('spmv-n-step')), `${control} ${value}: the graph that was kept still steps`)
    check((await chrome.evaluate(`document.getElementById('spmv-verdict').className`)).includes('spmv-refused'), `${control} ${value}: a refusal is styled apart`)
    // Rebuild 10 times, asked for while the request is still refused, does not take the graph that was kept away.
    const planKept = await text('spmv-n-plan')
    await click('spmv-rebuild')
    await pause(500)
    check((await sceneSvgs()) === 1 && (await marker()) === had.marker && (await text('spmv-n-plan')) === planKept, `${control} ${value}: Rebuild 10 times after a refusal leaves the graph that was kept`)
    check((await text('spmv-n-rebuild')) === '—' && (await text('spmv-verdict')).startsWith('Refused before anything was built'), `${control} ${value}: no rebuild was run, and the refusal is still what the readout says`)
    const stepKept = await text('spmv-n-step')
    await click('spmv-step')
    check((await text('spmv-n-step')) !== stepKept, `${control} ${value}: the graph that was kept after Rebuild still steps`)
    // With no graph there, a refusal builds nothing and says so.
    await click('spmv-teardown')
    await click('spmv-build')
    const bare = await text('spmv-verdict')
    check(bare.endsWith('Nothing was built.') && (await sceneSvgs()) === 0, `${control} ${value}: with no graph there, nothing is built and the refusal says so (${bare})`)
  }
  await choose('spmv-labels', '1')
  await choose('spmv-size', '2000000')
  const spent = await chrome.evaluate(`(() => { const t = performance.now(); document.getElementById('spmv-build').click(); return performance.now() - t })()`)
  check((await text('spmv-verdict')).includes('nodes 2000000 is over the budget'), 'a hostile size is refused')
  check(spent < 500, `a hostile size is answered at once (${spent.toFixed(0)} ms)`)

  // - - - A hub-heavy matrix: only a selected node's edges are strong, and the busiest column is dragged.
  await choose('spmv-size', '100')
  await choose('spmv-adjacency', 'hub')
  await click('spmv-build')
  check((await text('spmv-verdict')).startsWith('Accepted'), 'a hub-heavy matrix of 100 entities is accepted')
  check((await chrome.evaluate(`document.querySelector('svg.spmv-scene').classList.contains('spmv-focus')`)), 'above 60 edges only a selected node\'s are strong')
  await click('spmv-drag')
  const drag = await text('spmv-n-drag')
  check(/Column x\d+ has \d+ edges: 30 moves took .+ each/.test(drag), `the busiest column is dragged (${drag})`)

  // - - - Rebuilding leaves nothing behind.
  await click('spmv-rebuild')
  const rebuilt = await text('spmv-n-rebuild')
  check(rebuilt.includes('nothing was left behind'), `ten rebuilds leave the page as it was (${rebuilt.slice(0, 110)})`)
  check((await sceneSvgs()) === 1, 'a scene is left after the rebuilds')

  // - - - The largest size the budget allows, with its edges made readable.
  await choose('spmv-adjacency', 'flat')
  await choose('spmv-size', '10000')
  await click('spmv-build')
  check((await text('spmv-verdict')).startsWith('Accepted'), `10,000 entities are accepted (${(await text('spmv-verdict')).slice(0, 80)})`)
  await chrome.evaluate(`document.querySelector('[data-spmv-node="y7"]').dispatchEvent(new MouseEvent('click', { bubbles: true }))`)
  check((await text('spmv-selected')).split('\n').pop().startsWith('y7 = '), 'a node of a large graph can be selected too')

  // - - - Tear down.
  await click('spmv-teardown')
  check((await sceneSvgs()) === 0, 'tearing down removes the scene')
  check((await chrome.evaluate(`!!document.querySelector('svg#spmv-host foreignObject #spmv-build')`)), 'tearing down leaves the controls')
  check((await text('spmv-verdict')).startsWith('Torn down'), 'tearing down says so')

  // - - - A small scene again for the pictures.
  await choose('spmv-size', '16')
  await choose('spmv-adjacency', 'flat')
  await click('spmv-build')
  await chrome.evaluate(`document.querySelector('[data-spmv-node="y2"]').dispatchEvent(new MouseEvent('click', { bubbles: true }))`)
  await click('spmv-step')
  if (screenshot) {
    const shot = await chrome.send('Page.captureScreenshot', { format: 'png' })
    fs.writeFileSync(path.resolve(`${screenshot}-window.png`), Buffer.from(shot.result.data, 'base64'))
    console.log(`screenshot: ${screenshot}-window.png`)
  }
  check(thrown.length === 0, `nothing was thrown or logged as an error (${thrown.length}${thrown[0] ? `: ${thrown[0]}` : ''})`)
} finally {
  await chrome.close()
  server.close()
}
if (failures > 0) {
  console.log(`\n${failures} checks failed`)
  process.exit(1)
}
console.log('\nthe demo behaves')
