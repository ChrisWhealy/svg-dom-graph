//! What every sorting demo's browser tests share: reading the bars, their colours, the live region and the stepping
//! toolbar.
//!
//! The bars, their colours and the buttons are drawn by the shared `sorting` module of the demo, the same for every
//! algorithm, so what reads them back is the same too. A demo's tests call [`sort_demo_helpers!`] with the id of its
//! diagram, and get the functions below for it, such as `bars`, `values`, `with_fill`, `goto` and `click`. The macro
//! takes the `use` lines of the file that calls it, which needs `Tab`, `Duration` and `Instant`.

/// Defines, in the file that calls it, the constant `DIAGRAM` for the diagram whose selector is `$diagram`, the colours
/// the bars take, and the functions that read the bars and walk the toolbar.
macro_rules! sort_demo_helpers {
    ($diagram:literal) => {
        const DIAGRAM: &str = $diagram;

        /// Evaluates `expression` in the page and returns its string value.
        #[allow(dead_code)]
        fn text(tab: &Tab, expression: &str) -> Result<String, String> {
            tab.evaluate(expression, false)
                .map_err(|e| format!("could not evaluate {expression}: {e}"))?
                .value
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(|| format!("{expression} did not return a string"))
        }

        /// The colours the bars take, as the demo draws them. An ordinary bar has the library's usual light blue
        /// instead.
        #[allow(dead_code)]
        const RED: &str = "#e5534b";

        #[allow(dead_code)]
        const GREEN: &str = "#4cc86a";

        #[allow(dead_code)]
        const BLUE: &str = "#5b8def";

        #[allow(dead_code)]
        const TEAL: &str = "#a8dccf";

        #[allow(dead_code)]
        const LILAC: &str = "#d9c9f3";

        #[allow(dead_code)]
        const PALE: &str = "#eef1f5";

        /// One bar as the page draws it.
        #[allow(dead_code)]
        #[derive(serde::Deserialize, Debug)]
        struct Bar {
            value: u8,
            fill: String,
            height: f64,
            left: f64,
            bottom: f64,
        }

        /// The bars of the diagram from left to right. A bar is a node whose box is a rectangle 44 wide with its value
        /// in the text beside it, which tells it from the buttons and the note.
        ///
        /// They are in the order the scene drew them, which is the order of the array, and not sorted by where they are
        /// on the screen. A swap slides two bars for half a second, and where a bar is on the screen while it slides
        /// says nothing about the step. `left` is where it is now, so it is only meaningful when nothing is sliding.
        #[allow(dead_code)]
        fn bars(tab: &Tab) -> Result<Vec<Bar>, String> {
            let json = text(
                tab,
                &format!(
                    "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g')] \
                     .filter(g => g.children.length >= 2 && g.children[0].tagName === 'rect' && g.children[0].getAttribute('width') === '44' && g.children[1].tagName === 'text') \
                     .map(g => {{ const r = g.children[0].getBoundingClientRect(); \
                     return {{ value: Number(g.children[1].textContent), fill: g.children[0].getAttribute('fill'), \
                     height: r.height, left: r.left, bottom: r.bottom }}; }}))"
                ),
            )?;
            serde_json::from_str(&json).map_err(|e| format!("could not read the bars: {e}"))
        }

        /// The values of the bars from left to right.
        #[allow(dead_code)]
        fn values(tab: &Tab) -> Result<Vec<u8>, String> {
            Ok(bars(tab)?.iter().map(|bar| bar.value).collect())
        }

        /// The positions of the bars that have the fill `fill`.
        #[allow(dead_code)]
        fn with_fill(tab: &Tab, fill: &str) -> Result<Vec<usize>, String> {
            Ok(bars(tab)?
                .iter()
                .enumerate()
                .filter(|(_, bar)| bar.fill == fill)
                .map(|(i, _)| i)
                .collect())
        }

        /// What the live region outside the scene says, which is the one that goes with this diagram.
        #[allow(dead_code)]
        fn said(tab: &Tab) -> Result<String, String> {
            // The live region has the same prefix as the diagram: `quicksort-announcement` for `#quicksort-diagram`.
            let prefix = DIAGRAM.trim_start_matches('#').trim_end_matches("-diagram");
            text(
                tab,
                &format!("document.getElementById('{prefix}-announcement').textContent"),
            )
        }

        /// How many steps the walk has, read from the announcement of the step it is on: "Step 3 of 40. ..."
        #[allow(dead_code)]
        fn step_count(tab: &Tab) -> Result<usize, String> {
            let said = said(tab)?;
            said.split(" of ")
                .nth(1)
                .and_then(|rest| rest.split('.').next())
                .and_then(|n| n.trim().parse().ok())
                .ok_or_else(|| format!("could not read the number of steps from {said:?}"))
        }

        /// Presses the button drawn in the scene with `action`.
        #[allow(dead_code)]
        fn press(tab: &Tab, action: &str) -> Result<(), String> {
            tab.evaluate(
                &format!(
                    "document.querySelector('{DIAGRAM} g[data-action=\"{action}\"]') \
                     .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
                ),
                false,
            )
            .map_err(|e| format!("could not press {action}: {e}"))?;
            std::thread::sleep(Duration::from_millis(400));
            Ok(())
        }

        /// The walk's position, read from the hidden "Step" driver's own selection. `None` before the walk has started.
        #[allow(dead_code)]
        fn step_position(tab: &Tab) -> Result<Option<usize>, String> {
            let name = text(
                tab,
                &format!("(document.querySelector('{DIAGRAM} g[aria-label^=\"Step:\"]')?.getAttribute('aria-label') ?? '')"),
            )?;
            let Some((before, _)) = name.split_once(" selected") else {
                return Ok(None);
            };
            before
                .rsplit("cell ")
                .next()
                .and_then(|n| n.parse().ok())
                .map(Some)
                .ok_or_else(|| format!("could not read a cell from {name:?}"))
        }

        /// Waits until `step_position` returns something other than `from`, or fails after five seconds.
        #[allow(dead_code)]
        fn wait_for_move(tab: &Tab, from: Option<usize>) -> Result<(), String> {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if step_position(tab)? != from {
                    // Every move draws the scene again. Give the redraw its own moment to finish attaching.
                    std::thread::sleep(Duration::from_millis(100));
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(format!("the walk never left {from:?}"))
        }

        /// Presses a toolbar button with a real mouse click and waits for the walk to move.
        #[allow(dead_code)]
        fn click(tab: &Tab, name: &str) -> Result<(), String> {
            let from = step_position(tab)?;
            tab.find_element(&format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
                .map_err(|e| format!("could not find the {name:?} button: {e}"))?
                .click()
                .map_err(|e| format!("could not click {name:?}: {e}"))?;
            wait_for_move(tab, from)
        }

        /// Walks to `target` one step at a time.
        #[allow(dead_code)]
        fn goto(tab: &Tab, target: usize) -> Result<(), String> {
            for _ in 0..400 {
                let now = step_position(tab)?.unwrap_or(0);
                if now == target {
                    return Ok(());
                }
                click(tab, if now < target { "Next selection" } else { "Previous selection" })?;
            }
            Err(format!(
                "could not reach position {target}, stopped at {:?}",
                step_position(tab)?
            ))
        }

        /// Walks forward until the live region says `phrase`, one step at a time, and fails if it never does.
        #[allow(dead_code)]
        fn goto_note(tab: &Tab, phrase: &str) -> Result<(), String> {
            for _ in 0..200 {
                if said(tab)?.contains(phrase) {
                    return Ok(());
                }
                click(tab, "Next selection")?;
            }
            Err(format!("the walk never said {phrase:?}"))
        }

        /// How many bars have a slide on them.
        #[allow(dead_code)]
        fn slides(tab: &Tab) -> Result<usize, String> {
            text(
                tab,
                &format!("String(document.querySelectorAll('{DIAGRAM} animateTransform').length)"),
            )?
            .parse()
            .map_err(|e| format!("could not count the slides: {e}"))
        }

        /// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
        #[allow(dead_code)]
        fn view(tab: &Tab) -> Result<String, String> {
            text(
                tab,
                &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
            )
        }

    };
}

pub(crate) use sort_demo_helpers;
