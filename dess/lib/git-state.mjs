// Whether the source tree is as committed, for the evidence that records it.
//
// A run of the evidence scripts writes files under `dess/**/evidence/`, and a later step of the same run would read those as changes to the tree and
// record itself as measured on a dirty one. So the files that the scripts themselves write are not counted. Everything else is: a change to a source
// file, a test, a script, a workflow or a document makes the evidence dirty, and it is then not acceptance evidence.
import { spawnSync } from 'node:child_process'

/** The `git status --porcelain` lines of every change except those under `dess/**` `/evidence/`. Empty when the source tree is as committed. */
export const sourceChanges = root =>
  spawnSync('git', ['status', '--porcelain', '--', '.', ':(exclude,glob)dess/**/evidence/**'], {
    cwd: root,
    encoding: 'utf8'
  }).stdout.trim()

export const sourceDirty = root => sourceChanges(root).length > 0
