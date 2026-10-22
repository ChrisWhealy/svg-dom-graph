//! What the browser tests of a demo that steps share: reading the live region and walking the stepping toolbar, and for a
//! sorting demo, reading the bars and their colours.
//!
//! The walk is the same for every demo that steps with the selection toolbar, whatever it draws, so a demo's tests call
//! [`walk_helpers!`] with the id of its diagram, and get `text`, `said`, `step_count`, `step_position`, `goto` and
//! `click` for it. The bars, their colours and the buttons are drawn by the shared `sorting` module of the demo, the same
//! for every algorithm, so what reads them back is shared too. A sorting demo calls [`sort_demo_helpers!`] instead, which
//! gives it the same functions and `bars`, `values` and `with_fill` besides. The macros take the `use` lines of the file
//! that calls them, which needs `Tab`, `Duration` and `Instant`.

/// Defines, in the file that calls it, the constant `DIAGRAM` for the diagram whose selector is `$diagram`, and the
/// functions that read the live region and walk the stepping toolbar of a demo that steps: `text`, `said`, `step_count`,
/// `step_position`, `click`, `goto`, `goto_note` and `view`. Every demo that steps with the toolbar shares them, whatever
/// it draws. The file that calls it needs `Tab`, `Duration` and `Instant` in scope.
macro_rules! walk_helpers {
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

        /// What the live region outside the scene says, which is the one that goes with this diagram.
        #[allow(dead_code)]
        fn said(tab: &Tab) -> Result<String, String> {
            // The live region has the same prefix as the diagram: `quicksort-announcement` for `#quicksort-diagram`.
            let prefix = DIAGRAM.trim_start_matches('#').trim_end_matches("-diagram");
            text(tab, &format!("document.getElementById('{prefix}-announcement').textContent"))
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

        /// The walk's position, read from the hidden "Step" driver's own selection. `None` before the walk has started.
        #[allow(dead_code)]
        fn step_position(tab: &Tab) -> Result<Option<usize>, String> {
            let name = text(
                tab,
                &format!(
                    "(document.querySelector('{DIAGRAM} g[aria-label^=\"Step:\"]')?.getAttribute('aria-label') ?? '')"
                ),
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

        /// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
        #[allow(dead_code)]
        fn view(tab: &Tab) -> Result<String, String> {
            text(
                tab,
                &format!(
                    "(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"
                ),
            )
        }
    };
}

pub(crate) use walk_helpers;

/// Defines everything [`walk_helpers!`] does, and the colours the bars take, the functions that read the bars and press
/// the buttons drawn in the scene, and the checks every sorting demo is held to.
macro_rules! sort_demo_helpers {
    ($diagram:literal) => {
        $crate::sorting::walk_helpers!($diagram);

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

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // What every sorting demo is held to. Each demo's file has a test of its own that calls one of these, so that
        // the CI deals the tests out to the shards one by one, and a failure names the demo.
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

        /// The panel opens on ten bars in the order of `INITIAL`, standing on one baseline, as tall as their values in
        /// proportion, and in a row, with three buttons to choose another array and the walk at its start. None is
        /// marked.
        #[allow(dead_code)]
        fn opens_on_ten_bars_as_tall_as_their_values(tab: &Tab) -> Result<(), String> {
            let bars = bars(tab)?;
            check_eq(bars.iter().map(|b| b.value).collect::<Vec<_>>(), INITIAL.to_vec())?;
            for action in ["random", "sorted", "reversed"] {
                check_eq(
                    text(
                        tab,
                        &format!("String(document.querySelectorAll('{DIAGRAM} g[data-action=\"{action}\"]').length)"),
                    )?
                    .as_str(),
                    "1",
                )?;
            }
            let baseline = bars[0].bottom;
            let per_unit = bars[0].height / f64::from(bars[0].value);
            for bar in &bars {
                check(
                    (bar.bottom - baseline).abs() < 0.5,
                    &format!("{bar:?} is not on the baseline {baseline}"),
                )?;
                check(
                    (bar.height / f64::from(bar.value) - per_unit).abs() < 0.05,
                    &format!("{bar:?} is not as tall as its value, in proportion"),
                )?;
                check(bar.fill == bars[0].fill, "no bar is marked at the start")?;
            }
            check(bars.windows(2).all(|w| w[0].left < w[1].left), "the bars should be in a row")?;
            check_eq(step_position(tab)?, Some(0))?;
            check(said(tab)?.starts_with("Step 1 of "), "the first step should be announced")
        }

        /// Each of the first steps is announced with its number and the number of steps there are.
        #[allow(dead_code)]
        fn announces_each_step_with_how_far_along_it_is(tab: &Tab) -> Result<(), String> {
            let count = step_count(tab)?;
            for position in 1..6 {
                goto(tab, position)?;
                let said = said(tab)?;
                check(
                    said.starts_with(&format!("Step {} of {count}. ", position + 1)),
                    &format!("position {position} announced {said:?}"),
                )?;
            }
            Ok(())
        }

        /// The last step has the bars in order, every one in its final place, and says that the array is sorted.
        #[allow(dead_code)]
        fn ends_with_the_bars_in_order_and_all_in_place(tab: &Tab) -> Result<(), String> {
            let last = step_count(tab)? - 1;
            goto(tab, last)?;
            let mut expected = INITIAL;
            expected.sort_unstable();
            check_eq(values(tab)?, expected.to_vec())?;
            check_eq(with_fill(tab, LILAC)?, (0..10).collect::<Vec<_>>())?;
            check(said(tab)?.contains("sorted"), "the last step should say that it is sorted")
        }

        /// The zoom is kept when the scene is drawn again for a step, a restart, and another array.
        #[allow(dead_code)]
        fn keeps_the_zoom_across_every_kind_of_redraw(tab: &Tab) -> Result<(), String> {
            tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
                .map_err(|e| format!("could not find Zoom in: {e}"))?
                .click()
                .map_err(|e| format!("could not zoom in: {e}"))?;
            std::thread::sleep(Duration::from_millis(300));
            let zoomed = view(tab)?;
            check(!zoomed.is_empty(), "zooming in should change the view")?;
            goto(tab, 5)?;
            check_eq(view(tab)?, zoomed.clone())?;
            goto(tab, 2)?;
            click(tab, "Restart selection")?;
            check_eq(view(tab)?, zoomed.clone())?;
            press(tab, "reversed")?;
            check_eq(view(tab)?, zoomed)
        }

        /// "Reverse sorted" gives ten bars from the tallest to the shortest.
        #[allow(dead_code)]
        fn reversed_button_gives_bars_from_tallest_to_shortest(tab: &Tab) -> Result<(), String> {
            press(tab, "reversed")?;
            let values = values(tab)?;
            check_eq(values.len(), 10)?;
            check(
                values.windows(2).all(|w| w[0] > w[1]),
                &format!("not in descending order: {values:?}"),
            )
        }

        /// "Random array" gives ten different bars.
        #[allow(dead_code)]
        fn random_button_gives_ten_different_bars(tab: &Tab) -> Result<(), String> {
            press(tab, "random")?;
            let mut values = values(tab)?;
            check_eq(values.len(), 10)?;
            values.sort_unstable();
            values.dedup();
            check_eq(values.len(), 10)
        }
    };
}

pub(crate) use sort_demo_helpers;
