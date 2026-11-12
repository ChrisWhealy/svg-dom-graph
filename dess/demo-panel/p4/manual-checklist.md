# Manual checks

These are for a person with the tool in front of them, since a script cannot hear a screen reader or hold a phone.
Each one says what to do, what is expected, and what to write down.
Record the result in `evidence/manual-observations.md`, with the date, the browser and its version, and the tool and its version.
A check that was not done is left blank, and is never a pass.

Open the staged gallery (`cargo run -p demo-server -- --build-only`, then serve `target/demo-stage`) at `#panel-service-simulator`.

## VoiceOver, Safari, macOS

Turn VoiceOver on with Command and F5.

1. Move to the controls with VoiceOver's own keys, and listen to each.
   Expected: Previous, Next, Play, Restart, Scenario, Snapshot and Speed, each with its role.
2. Press Next.
   Expected: the live region announces the new snapshot, and the focus stays on Next.
3. Move to the range and press the arrow keys.
   Expected: "Snapshot", the number, and whether it is announced as "N of M" or only as N.
   Write down which, since Chrome's tree reports only the number.
4. Choose another scenario from the list.
   Expected: the new scenario's first snapshot is announced, and an error, if there is one, is read out.
5. Press Play, then Pause.
   Expected: the button's name or state changes, and each tick is announced only as often as the page says.
6. Move to the scene itself.
   Expected: the servers and the data node are named, and the inspector is reachable.

## NVDA, Firefox and Chrome, Windows

Repeat the six steps.
Write down where NVDA's browse mode and its focus mode differ for the range and the list.

## A phone, Safari on iOS and Chrome on Android

1. Open the panel.
   Write down how wide the scene is on the screen and the height of the smallest control.
   Expected by the script's measurements: it is far too small to touch.
2. Touch each control.
   Write down which could be hit.
3. Turn the phone sideways, and write down the same.
4. Choose a scenario with the list.
   Write down whether the native chooser opens and whether the choice reaches the page.

## Safari, keyboard only

Safari does not Tab to buttons unless Option is held, or the setting "Press Tab to highlight each item on a webpage" is on.
Do the keyboard path once with the setting on, and once with Option and Tab, and write down the order.
The script saw Tab go to the speed list and then out of the scene.
