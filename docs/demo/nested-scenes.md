# Nested Scenes

A container node looks like any other node, but it owns a nested `Scene` of its own.
`Scene::enter` and `Scene::exit` navigate into and out of it, hiding and showing each `<svg>` root as they go.
This keeps a diagram small at the top level, while a step that is genuinely complex still gets its own full-size view one level down.

The "Nested Scenes" panel uses SHA3's `Theta` function, which contains three sub-functions.
They are `ThetaC`, `ThetaD`, and a final `XOR` loop that folds the `ThetaD` result back into the state array.
Each is a container node.
Clicking one drills into it, and a rounded, bordered frame marks it as a nested view with a close button in its corner.
`ThetaC`'s nested scene is the same operator chain the Cell Selection demo draws.

The Next and Prev buttons below the diagram move the focus from one function to the next.
Only the function in focus can be clicked.
All three are real, and `Theta Output` holds zeros until the walk reaches the `XOR` loop.
The input array here is made-up data.
