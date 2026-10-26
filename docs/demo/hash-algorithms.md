# Hash Algorithms

The three hash demos each hash one short message.
Every value on screen is a real result of that one calculation, worked out in plain Rust.
The library never computes anything: each demo works out a value and then hands it to `svg-dom-graph` to draw.

Each demo is large, so its gallery panel keeps the description and a button.
The button opens the scene in a window of its own, sized to fit it.
A selection toolbar steps through the algorithm, and a stride button moves eight steps at a time.
A node the walk has not reached yet is drawn faint by `Scene::set_unreached_cells`, so a placeholder zero cannot be mistaken for a computed one.
Within a stage, every step only writes new values and marks into the nodes already drawn, with `Scene::set_data_values`.
Stepping backwards is the same write as stepping forwards.
Crossing from one stage to the next draws the diagram again, and keeps the zoom and pan with `Scene::view` and `Scene::set_view`.

## SHA1

SHA-1 hashes a message in 64-byte blocks, and this demo needs only one.

The diagram draws only the stage in progress.

* **The message schedule.**<br>
  The message, shown as plain text, sits above its block of 16 words.
  The block feeds an 80-word schedule, shown as a grid.
  Each word after the 16th is the XOR of four earlier words rotated left by one bit.
  While a word is being built, the "XOR" node shows the four words it reads, and their XOR.
  `LabellingStyle::Text` labels the four words `ms[n-3]`, `ms[n-8]`, `ms[n-14]` and `ms[n-16]`.
  The node below it is that XOR rotated left by one bit, which is the new schedule word.
  The word being built is the one in focus in the schedule, and the four it reads are marked as secondary cells.
* **Compression.**<br>
  The 80 rounds are drawn in four sections of 20, and the diagram names them.
  Every round in a section uses the same round function and the same round constant.
  Crossing into the next section draws the diagram again.
  Each round takes the working variables `a` to `e`, and adds five terms.
  They are `ROTL(a, 5)`, the round function of `b`, `c` and `d`, `e`, the round constant and the schedule word `W`.
  `W` is the word marked in the schedule above, which is named as such.
  The initial hash is fixed by the standard and is not worked out from the message, which the node's name says.
  After the last round, the digest is each working variable added to its initial hash word.
  The last step writes it out as the 40-digit hash.

## SHA2-256

SHA-256 hashes a message in 64-byte blocks, and this demo needs only one.

The diagram draws only the half in progress.
While the message schedule is built, it shows the message, its block and the 64-word schedule.
It also shows the four earlier words and two sigma functions that make each new word.
Once the schedule is complete, it adds these columns beside the schedule:

* the initial hash and the working variables
* the terms of `temp1` and `temp2`
* the next working variables and the 64 round constants

After 64 rounds, the digest is each working variable added to its initial hash word.
A last step writes it out as the 64-digit hash.

Words are shown in binary, so that the rotations and shifts at the heart of the algorithm are visible.

## SHA3 Sponge

SHA-3 is a sponge function: it absorbs its input into an internal state, then squeezes the output out.
This demo computes the SHA3-256 hash of a short message in a single call to `Keccak-f[1600]`.

The top-level diagram shows the input block being XOR'ed into the rate, and the permutation.
It also shows the first 256 bits of the result, which are the hash.
The `Keccak-f` function is a container node: when it has focus, marked with a red border, it can be entered.
Within `Keccak-f` there are the 24 rounds.
Each round has the five steps `Theta`, `Rho`, `Pi`, `Chi` and `Iota`.
Within `Theta` are the `ThetaC`, `ThetaD` and `XOR` loop scenes of the Nested Scenes demo.

Every nested scene shows the correct state when it is entered, because each value is computed from the one before it.
So stepping over a function gives exactly what stepping into it would have ended on.
