// Shows that the differential run and the browser tests can fail: the Rust core and the scene are broken in small ways, one at a time,
// and the differential run or the tests in Chrome have to find each one. A comparison that agrees proves nothing until it is known to
// notice a disagreement.
//
//   node spmv/mutation-check.mjs [--out FILE]
//
// Each mutation is a single edit to a source file of `spmv-core` or `spmv-scene`, made on a copy that is put back afterwards, even if
// the run is interrupted. A core mutation is found by the differential run, and a scene mutation by `wasm-pack test --headless --chrome
// spmv-scene --test browser`, which needs Chrome. A run takes a few minutes.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '..')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null
const CORE = 'spmv-core/src/'
const SCENE = 'spmv-scene/src/'
const src = file => path.join(root, file)

const MUTATIONS = [
  ['a duplicate column is kept in a row', CORE + 'matrix.rs', 'if !chosen.contains(&c) {', 'if true {'],
  [
    'a hub draw needs u less than or equal to the percentage',
    CORE + 'matrix.rs',
    'if u < spec.hub_percent &&',
    'if u <= spec.hub_percent &&'
  ],
  [
    'a value has the wrong sign',
    CORE + 'matrix.rs',
    'let negative = values.below(2) == 1;',
    'let negative = values.below(2) == 0;'
  ],
  ['the values are drawn before the columns are sorted', CORE + 'matrix.rs', 'chosen.sort_unstable();', ''],
  ['x starts one too low', CORE + 'run.rs', 'as i64 - 99).collect();', 'as i64 - 98).collect();'],
  [
    'an update may reach beyond the window',
    CORE + 'run.rs',
    'self.stream.below(self.cells) as usize',
    'self.stream.below(self.matrix.cols as u64) as usize'
  ],
  [
    'the incremental product skips the last nonzero of a column',
    CORE + 'run.rs',
    'for k in matrix.col_ptr[j]..matrix.col_ptr[j + 1] {',
    'for k in matrix.col_ptr[j]..matrix.col_ptr[j + 1].saturating_sub(1).max(matrix.col_ptr[j]) {'
  ],
  [
    'a changed entry is not stored in x',
    CORE + 'run.rs',
    '            self.x[j] = v;\n',
    '            self.x[j] = self.x[j];\n'
  ],
  ['a limit is exclusive and not inclusive', CORE + 'budget.rs', 'if value > limit {', 'if value >= limit {'],
  [
    'the degree check ignores the columns',
    CORE + 'budget.rs',
    'stats.max_row_degree.max(stats.max_col_degree)',
    'stats.max_row_degree'
  ],
  [
    'the first stage bounds the edges by the rows alone',
    CORE + 'budget.rs',
    'spec.rows.saturating_mul(spec.per_row)',
    'spec.rows'
  ],
  [
    'the matrix hash leaves out the values',
    CORE + 'matrix.rs',
    'w.u64(c as u64).i64(v);',
    'w.u64(c as u64).i64(0 * v);'
  ],
  ['labels are padded with another character', CORE + 'matrix.rs', "text.push('-');", "text.push('_');"],
  [
    'a label is cut to its length',
    CORE + 'matrix.rs',
    '    text\n}\n',
    '    text.truncate(len.max(1));\n    text\n}\n'
  ],
  [
    'a step writes the x values into the y data node',
    SCENE + 'scene.rs',
    'NodeValues::I64(run.y[..self.cells].to_vec())',
    'NodeValues::I64(run.x[..self.cells].to_vec())',
    'browser'
  ],
  [
    'the edges leave out the first nonzero of a row',
    SCENE + 'scene.rs',
    'for (j, value) in matrix.row(i) {',
    'for (j, value) in matrix.row(i).skip(1) {',
    'browser'
  ],
  [
    'the square of the pattern leaves out the first nonzero of a row',
    SCENE + 'scene.rs',
    'for (j, _) in matrix.row(i) {',
    'for (j, _) in matrix.row(i).skip(1) {',
    'browser'
  ],
  [
    'a node is labelled with the next index',
    SCENE + 'scene.rs',
    'label(kind, index, len)',
    'label(kind, index + 1, len)',
    'browser'
  ],
  ['a teardown leaves the host in the page', SCENE + 'scene.rs', 'element.remove();', 'let _ = element;', 'browser'],
  [
    'a move does not move the node',
    SCENE + 'scene.rs',
    'Point::new(at.x + dx, at.y + dy)',
    'Point::new(at.x, at.y)',
    'browser'
  ],
  ['the square of the pattern is not added', SCENE + 'scene.rs', 'add_spy(svg_id, prepared, &layout)?;', '', 'browser'],
  [
    'the data nodes are given less room than they take',
    SCENE + 'layout/mod.rs',
    'pub const CELL_PITCH: (f64, f64) = (50.0, 37.0);',
    'pub const CELL_PITCH: (f64, f64) = (50.0, 20.0);',
    'browser'
  ],
  [
    'an edge of a negative value is drawn as a positive one',
    SCENE + 'scene.rs',
    'let sign = if value < 0 { "spmv-neg" } else { "spmv-pos" };',
    'let sign = if value < 0 { "spmv-pos" } else { "spmv-neg" };',
    'browser'
  ],
  ['a selection does not put out the last one', SCENE + 'scene.rs', 'remove_class(&self.edge_paths[k], "spmv-on")?;', 'let _ = k;', 'browser'],
  [
    'selecting a column leaves out its first edge',
    SCENE + 'scene.rs',
    'matrix.col(j).filter_map(|(i, _)| matrix.edge_index(i, j)).collect()',
    'matrix.col(j).filter_map(|(i, _)| matrix.edge_index(i, j)).skip(1).collect()',
    'browser'
  ],
  [
    'a small graph is not drawn as single columns',
    SCENE + 'layout/mod.rs',
    'let single = spec.cols <= SINGLE_MAX && spec.rows <= SINGLE_MAX;',
    'let single = false;',
    'scene-unit'
  ],
  [
    'a negative term is written with a plus sign',
    CORE + 'explain.rs',
    '(_, true) => format!("− {}·x{c}", -v),',
    '(_, true) => format!("+ {}·x{c}", -v),',
    'core-unit'
  ],
  [
    'a long row does not say how many terms it left out',
    CORE + 'explain.rs',
    'let tail = if more > 0 { format!(" and {more} more terms") } else { String::new() };',
    'let tail = String::new();',
    'core-unit'
  ],
  ['the index of an edge is one too high', CORE + 'matrix.rs', 'Some(range.start + at)', 'Some(range.start + at + 1)', 'core-unit'],
  ['the captions of the three boxes are not added', SCENE + 'scene.rs', 'add_captions(svg_id, prepared, &layout)?;', '', 'browser'],
  ['the cells of a selected row are not lit', SCENE + 'scene.rs', 'add_class(&self.matrix_cells[*k], "spmv-on")?;', 'let _ = k;', 'browser'],
  ['the entries of x that a row uses are not marked', SCENE + 'scene.rs', 'Selection::Row(i) => (matrix.row(i).map(|(j, _)| j).collect(), vec![i]),', 'Selection::Row(i) => (Vec::new(), vec![i]),', 'browser'],
  ['a step does not mark what it changed', SCENE + 'scene.rs', 'if now[n] != before[n] {', 'if false {', 'browser'],
  ['the value beside a node is not updated by a step', SCENE + 'scene.rs', 'value.set_text_content(Some(&new));', '', 'browser'],
  ['a column story multiplies by the wrong entry', CORE + 'explain.rs', 'bracketed(*v),\n            bracketed(x[j]),', 'bracketed(*v),\n            bracketed(*v),', 'core-unit'],
  ['a step does not write the products of the worked row', SCENE + 'worked/mod.rs', 'set(term.product, vec![*product])?;', '', 'browser'],
  ['the terms past the limit are left out of the folded sum by one', SCENE + 'worked/mod.rs', 'terms[MAX_TERMS..].iter()', 'terms[MAX_TERMS + 1..].iter()', 'scene-unit'],
  ['the running total begins at one', SCENE + 'worked/mod.rs', 'let mut total = 0;', 'let mut total = 1;', 'scene-unit'],
  ['the cells of a row of A are tagged with the first row', SCENE + 'worked/mod.rs', '            tag(cell, i)?;\n            add_class(cell, "spmv-a-cell")?;', '            tag(cell, 0)?;\n            add_class(cell, "spmv-a-cell")?;', 'browser'],
  ['in-order steps stay on the first entry', CORE + 'run.rs', 'self.cursor = (j + 1) % window;', 'self.cursor = j;', 'core-unit'],
  ['a negative bar grows to the right', SCENE + 'worked/mod.rs', 'let x = if *v < 0 { self.bar_zero - width } else { self.bar_zero };', 'let x = self.bar_zero;', 'browser'],
  ['the marks of the last step are not put out', SCENE + 'worked/mod.rs', '            remove_class(&cell, "spmv-fed")?;', '            let _ = &cell;', 'browser'],
  ['the caption of the data node is put on top of it', SCENE + 'layout/mod.rs', 'x_caption: (value_room, data_bottom + CAPTION_LINE),', 'x_caption: (value_room, 0.0),', 'scene-unit']
]

const files = new Map(MUTATIONS.map(([, f]) => [f, fs.readFileSync(src(f), 'utf8')]))
// Every mutation has to apply to the source as it is now, and rustfmt can move the text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([, f, from]) => !files.get(f).includes(from)).map(([name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const build = () => spawnSync('cargo', ['build', '--release', '-p', 'spmv-core'], { cwd: root, encoding: 'utf8' })
const differential = () =>
  spawnSync('node', ['spmv/differential.mjs', '--seeds', '150'], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 })
const browser = () =>
  spawnSync('wasm-pack', ['test', '--headless', '--chrome', 'spmv-scene', '--test', 'browser'], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    timeout: 900_000
  })

const results = []
try {
  for (const [name, f, from, to, kind = 'differential'] of MUTATIONS) {
    fs.writeFileSync(src(f), files.get(f).replace(from, to))
    if (kind === 'core-unit' || kind === 'scene-unit') {
      const crate = kind === 'core-unit' ? 'spmv-core' : 'spmv-scene'
      const ran = spawnSync('cargo', ['test', '-p', crate, '--lib'], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 })
      fs.writeFileSync(src(f), files.get(f))
      const text = `${ran.stdout}${ran.stderr}`
      const failed = text
        .split('\n')
        .filter(line => line.startsWith('test ') && line.endsWith(' FAILED'))
        .map(line => line.slice(5).split(' ... ')[0])
      results.push({ name, kind, caught: failed.length > 0, detail: failed.length > 0 ? failed.join(', ') : 'no test failed' })
      console.log(`${failed.length > 0 ? 'caught ' : 'MISSED '} ${name}${failed.length > 0 ? `\n         first test to fail: ${failed[0]}` : ''}`)
      continue
    }
    if (kind === 'browser') {
      const ran = browser()
      fs.writeFileSync(src(f), files.get(f))
      const text = `${ran.stdout}${ran.stderr}`
      const failed = text
        .split('\n')
        .filter(line => line.startsWith('test ') && (line.endsWith(' FAILED') || line.endsWith(' FAIL')))
        .map(line => line.slice(5).split(' ... ')[0])
      const built = !text.includes('could not compile') && !text.includes('Compilation of your program failed')
      results.push({
        name,
        kind,
        caught: failed.length > 0,
        detail: failed.length > 0 ? failed.join(', ') : built ? 'no test failed' : 'did not build'
      })
      console.log(
        `${failed.length > 0 ? 'caught ' : 'MISSED '} ${name}${failed.length > 0 ? `\n         first test to fail: ${failed[0]}` : ''}`
      )
      continue
    }
    const built = build()
    fs.writeFileSync(src(f), files.get(f))
    if (built.status !== 0) {
      results.push({
        name,
        kind,
        caught: false,
        detail: `did not build: ${built.stderr.split('\n').find(l => l.startsWith('error'))}`
      })
      console.log(`DID NOT BUILD  ${name}`)
      continue
    }
    const ran = differential()
    const first = ran.stdout
      .split('\n')
      .find(l => l.startsWith('DIFFERENT '))
      ?.slice(10, 200)
    results.push({ name, kind, caught: ran.status === 1, detail: first ?? 'no difference' })
    console.log(
      `${ran.status === 1 ? 'caught ' : 'MISSED '} ${name}${first ? `\n         first difference: ${first}` : ''}`
    )
  }
} finally {
  for (const [f, text] of files) fs.writeFileSync(src(f), text)
  build()
}
const missed = results.filter(r => !r.caught)
console.log(
  missed.length
    ? `\n${missed.length} mutation(s) were not caught: the check cannot be trusted`
    : `\n${results.length} of ${results.length} mutations were caught, and the sources are restored`
)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify({ mutations: results }, null, 2)}\n`)
process.exit(missed.length ? 1 : 0)
