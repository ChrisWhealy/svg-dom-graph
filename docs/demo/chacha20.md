# ChaCha20

ChaCha20 is a stream cipher defined by RFC 8439.
It turns a 256-bit key, a 96-bit nonce and a 32-bit block counter into a stream of pseudo-random bytes.
It then XORs that keystream into the message.
Decrypting is the same operation as encrypting.

The demo walks through all 20 rounds of the block function, one quarter round at a time, and ends at the keystream block.
Every value on screen is a real result, worked out in plain Rust.
The library never computes anything: the demo works out each value and then hands it to `svg-dom-graph` to draw.

The scene is large, so the gallery panel keeps the description and a button.
The button opens the scene in a window of its own, sized to fit it.

## The state and the rounds

The state is sixteen 32-bit words in a 4 by 4 matrix.
They are four constants, the eight words of the key, the counter, and three words of nonce.
Each round applies the quarter round to four words of the state.
An odd round uses the four columns, and an even round uses the four diagonals.
There are 20 rounds, so 80 quarter rounds.
The first state is the example in RFC 8439, section 2.3.2.

## The diagram

* **The state before.**<br>
  At the top is the state the quarter round starts from.
  Its rows are labelled `a` to `d`, after the four words a column or diagonal holds.
  The four words this quarter round mixes are marked as secondary cells.
* **The four columns.**<br>
  The quarter round mixes four words, `a`, `b`, `c` and `d`, and each runs down a column of its own.
  The four words marked in the state feed the named nodes at the top.
  The named nodes at the bottom hold the four words that come out.
* **The twelve steps.**<br>
  Each step is an operator node in the column of the word it updates.
  `ADD` is `+=`, `XOR` is `^=`, and `ROTL` is `<<<=`, a rotation left.
  An `ADD` or `XOR` has a second operand, which is the latest value of another word, so its connector crosses from that word's column.
  A rotation has the one operand.
  To save vertical space, steps that hand a value straight on share a row.
  The first `ADD` and the `XOR` that follows it are on one row.
  So are each rotation of `d` and the `ADD` and `XOR` after it.
  The `ADD` under `a` has fixing points on its east side, one where the operand from `b` arrives and one where the sum leaves.
* **The state after.**<br>
  Below the columns is the state after the quarter round: the state before, with the four new words in place of the four it mixed.
* **The list of steps.**<br>
  At the right is a list of the 12 steps, each showing the value it produced.
* **The keystream block.**<br>
  After the last quarter round, one more step shows the state after 20 rounds and the initial state.
  It also shows the keystream block that they make when added word by word.
  This is the block that RFC 8439, section 2.3.2, publishes.

## Stepping

The scene's selection toolbar walks through the rounds.
"Next" and "Prev" move one step, and when a quarter round ends they move on to the next one.
The "Round" buttons move one whole round, which is four quarter rounds.
The title names the round, and says whether the quarter round mixes a column or a diagonal.

The current step and the nodes it reads share the secondary highlight.
Anything the walk has not reached yet shows zero and is drawn faint, so a placeholder zero is not mistaken for a result.
"Restart" returns every node to that state.

Every quarter round has the same layout, so one diagram serves all 80.
Stepping only writes new values and marks into the nodes already drawn, with `Scene::set_data_values`.
That method accepts an operator node's result as well as a data node's.
Which values and marks a step shows is a pure function of the position, so stepping backwards is the same write as stepping forwards.
Only crossing to the keystream block, or back, draws the diagram again, keeping the zoom and pan.

## What is tested

The tests in `demo-app/src/chacha20/algorithm/unit_tests.rs` check the quarter round, the initial state, the block function and a complete encryption against the values RFC 8439 publishes.
They also check that the 80 quarter rounds chain from one to the next, and that odd rounds mix columns and even rounds mix diagonals.
The tests in `demo-app/src/chacha20/walk/unit_tests.rs` check what each position of the walk shows.
The encryption and the keystream bytes are not drawn, so only the tests call them.
