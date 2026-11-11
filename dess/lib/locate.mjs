// Finds the file that holds a piece of source text, for tools that edit a source file by its text.
//
// A module that was split into a folder keeps its code in several files, and an edit that named the one file now has to be found in whichever file
// holds the text. The file that was named is tried first, then every Rust file of its folder, the unit tests excepted.
import fs from 'node:fs'
import path from 'node:path'

const rustFiles = dir =>
  fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap(entry =>
      entry.isDirectory()
        ? rustFiles(path.join(dir, entry.name))
        : entry.name.endsWith('.rs')
          ? [path.join(dir, entry.name)]
          : []
    )

/** The path, relative to `root`, of the file that holds `text`: `file` itself, or the one file of its folder that does. `file` is returned when none or several do, so that the caller reports it as it always has. */
export const locate = (root, file, text) => {
  const named = path.join(root, file)
  if (fs.existsSync(named) && fs.readFileSync(named, 'utf8').includes(text)) return file
  const folder = file.endsWith('/mod.rs') ? path.dirname(named) : named.replace(/\.rs$/, '')
  if (!fs.existsSync(folder)) return file
  const found = rustFiles(folder).filter(
    candidate => !candidate.includes(`${path.sep}unit_tests`) && fs.readFileSync(candidate, 'utf8').includes(text)
  )
  return found.length === 1 ? path.relative(root, found[0]) : file
}

/** Each mutation is `[file, name, from, ...]`, and the file is replaced by the one that holds `from`. */
export const locating = root => ([file, name, from, ...rest]) => [locate(root, file, from), name, from, ...rest]
