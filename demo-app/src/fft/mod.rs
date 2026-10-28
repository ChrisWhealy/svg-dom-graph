//! The radix-2 FFT panel:
//!
//! - `panel-fft` / `#fft-wave-plot`, `#fft-overview-diagram`, `#fft-inspector-diagram` — [`build_fft_demo`]: three sine
//!   waves with sliders for their amplitudes and phases, a graph of the waves and their sum, a forward,
//!   decimation-in-time fast Fourier transform of eight or sixteen samples of that sum, and what the transform found.

mod algorithm;
mod cursor;
mod inspector;
mod layout;
mod overview;
mod plot;
mod report;
mod sizing;
mod waves;
mod words;

use crate::source_file;
use algorithm::{FftTrace, SIZES, trace};
use cursor::{Cursor, Outcome, apply_pair, apply_transition, retry, shown_stage};
use demo_common::{
    focus,
    util::{announce, clear_error, document, ensure_svg_in, report_error, required_element, required_input, stringify},
};
use inspector::Inspector;
use overview::Overview;
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, SelectionTransition},
};
use wasm_bindgen::{JsCast, closure::Closure};
use waves::{Findings, Wave};
use web_sys::{Element, HtmlSelectElement};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the arithmetic, then where everything goes, then what each part draws and says.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("fft/waves/mod.rs", "waves/mod.rs"),
    source_file!("fft/plot/mod.rs", "plot/mod.rs"),
    source_file!("fft/report/mod.rs", "report/mod.rs"),
    source_file!("fft/algorithm/mod.rs", "algorithm/mod.rs"),
    source_file!("fft/layout/mod.rs", "layout/mod.rs"),
    source_file!("fft/sizing/mod.rs", "sizing/mod.rs"),
    source_file!("fft/overview.rs", "overview.rs"),
    source_file!("fft/inspector.rs", "inspector.rs"),
    source_file!("fft/cursor/mod.rs", "cursor/mod.rs"),
    source_file!("fft/words/mod.rs", "words/mod.rs"),
    source_file!("fft/mod.rs", "mod.rs"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ids of the panel's own elements.
const OVERVIEW_STAGE: &str = "fft-overview-stage";
const OVERVIEW_ID: &str = "fft-overview-diagram";
const INSPECTOR_STAGE: &str = "fft-inspector-stage";
const INSPECTOR_ID: &str = "fft-inspector-diagram";
const LENGTH_ID: &str = "fft-length";
const PLOT_STAGE: &str = "fft-wave-plot";
const PLOT_SVG: &str = "fft-wave-diagram";
const FINDINGS_ID: &str = "fft-findings";
const PAIR_ID: &str = "fft-pair";
const STATUS_ID: &str = "fft-status";
const ERROR_ID: &str = "fft-error";
const RETRY_ID: &str = "fft-retry";
const TITLE_ID: &str = "fft-inspector-title";

/// The ids of the two sliders of each wave and of the two numbers beside them: amplitude, phase, and then their
/// outputs.
const WAVE_CONTROLS: [[&str; 4]; 3] = [
    ["fft-a-amplitude", "fft-a-phase", "fft-a-amplitude-out", "fft-a-phase-out"],
    ["fft-b-amplitude", "fft-b-phase", "fft-b-amplitude-out", "fft-b-phase-out"],
    ["fft-c-amplitude", "fft-c-phase", "fft-c-amplitude-out", "fft-c-phase-out"],
];

/// The ids a replacement scene is drawn under, while the old one stays on the page. They become the ids above once the
/// replacement has been drawn.
const STAGING_OVERVIEW_ID: &str = "fft-overview-staging";
const STAGING_INSPECTOR_ID: &str = "fft-inspector-staging";
/// Each replacement scene's staging id, with the id it takes once it is on the page.
const PROMOTIONS: [(&str, &str); 2] = [(STAGING_OVERVIEW_ID, OVERVIEW_ID), (STAGING_INSPECTOR_ID, INSPECTOR_ID)];
/// Keeps a scene that is being drawn out of sight, but still laid out, so that text can be measured.
const STAGING_STYLE: &str = "position: fixed; left: -30000px; top: 0; visibility: hidden; pointer-events: none;";

/// The size of the `<svg>` before its scene has measured itself.
fn provisional() -> Size {
    Size::new(1000.0, 600.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything drawn for one size and one set of waves. Replaced as a whole when either changes.
struct Live {
    /// Which build is this? A callback of a scene that has been replaced carries an older number, and is ignored.
    generation: u64,
    n: usize,
    waves: [Wave; 3],
    trace: FftTrace,
    overview: Overview,
    inspector: Inspector,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Default)]
struct Runtime {
    generation: u64,
    live: Option<Rc<Live>>,
    /// Taken out while a callback uses it, so that nothing can borrow it twice.
    cursor: Option<Cursor>,
}

type Listener = Closure<dyn FnMut(web_sys::Event)>;

/// The room round the plot node in its own scene.
const PLOT_MARGIN: f64 = 8.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
    // The scene that holds the graph of the waves, and its plot node. It is drawn once, and only what is plotted
    // changes.
    static WAVE_PLOT: RefCell<Option<(Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The build with this generation number, if it is still the one on the page.
fn current(generation: u64) -> Option<Rc<Live>> {
    RUNTIME.with_borrow(|runtime| runtime.live.clone().filter(|live| live.generation == generation))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The build on the page, whatever its number.
fn latest() -> Option<Rc<Live>> {
    RUNTIME.with_borrow(|runtime| runtime.live.clone())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn take_cursor() -> Option<Cursor> {
    RUNTIME.with_borrow_mut(|runtime| runtime.cursor.take())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn put_cursor(cursor: Cursor) {
    RUNTIME.with_borrow_mut(|runtime| runtime.cursor = Some(cursor));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `<select>` with this id.
fn select(id: &str) -> Result<HtmlSelectElement, String> {
    required_element(&document()?, id)?
        .dyn_into::<HtmlSelectElement>()
        .map_err(|_| format!("#{id} must be a <select>"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size the length list names, or the first size offered if it names none.
fn chosen_size() -> Result<usize, String> {
    let value = select(LENGTH_ID)?.value();
    Ok(value.parse().ok().filter(|n| SIZES.contains(n)).unwrap_or(SIZES[0]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The waves the sliders show. A slider that cannot be read counts as zero.
fn chosen_waves() -> Result<[Wave; 3], String> {
    let document = document()?;
    let mut waves = waves::initial();
    for (wave, [amplitude, phase, ..]) in waves.iter_mut().zip(WAVE_CONTROLS) {
        let amplitude = required_input(&document, amplitude)?.value_as_number();
        let phase = required_input(&document, phase)?.value_as_number();
        *wave = Wave::new(amplitude, phase);
    }
    Ok(waves)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the sliders at `waves`.
fn set_sliders(waves: &[Wave; 3]) -> Result<(), String> {
    let document = document()?;
    for (wave, [amplitude, phase, ..]) in waves.iter().zip(WAVE_CONTROLS) {
        required_input(&document, amplitude)?.set_value(&wave.amplitude.to_string());
        required_input(&document, phase)?.set_value(&wave.phase.to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the waves, their sum and the samples on the graph. The graph is a scene of its own with one plot node in it,
/// drawn the first time and kept, so a slider that moves only replaces what is plotted.
fn draw_plot(waves: &[Wave; 3], n: usize) -> Result<(), String> {
    let existing = WAVE_PLOT.with_borrow(Clone::clone);
    let (scene, node) = match existing {
        Some(drawn) => drawn,
        None => {
            let document = document()?;
            let (width, height) = plot::SIZE;
            let size = Size::new(width + 2.0 * PLOT_MARGIN, height + 2.0 * PLOT_MARGIN);
            ensure_svg_in(&document, PLOT_STAGE, PLOT_SVG, size)?;
            let scene = Scene::new(SvgRoot::attach(PLOT_SVG).map_err(stringify)?).map_err(stringify)?;
            let node = scene
                .add_plot_node(Point::new(PLOT_MARGIN, PLOT_MARGIN), plot::config())
                .map_err(stringify)?;
            WAVE_PLOT.with_borrow_mut(|slot| *slot = Some((scene.clone(), node)));
            (scene, node)
        },
    };
    scene.set_plot_description(node, &plot::label(waves, n)).map_err(stringify)?;
    scene.set_plot_items(node, &plot::items(waves, n)).map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows the waves: the graph, the numbers beside the sliders, and what a transform of `n` samples of their sum finds.
/// Nothing in the scenes changes.
fn show_waves(waves: &[Wave; 3], n: usize) -> Result<Findings, String> {
    let fft = trace(&waves::samples(waves, n)).map_err(|e| e.to_string())?;
    let spectrum = fft.states.last().ok_or("the transform has no result")?;
    let found = waves::findings(spectrum);
    let document = document()?;
    draw_plot(waves, n)?;
    required_element(&document, FINDINGS_ID)?.set_inner_html(&report::markup(waves, &found));
    for (wave, [.., amplitude, phase]) in waves.iter().zip(WAVE_CONTROLS) {
        required_element(&document, amplitude)?.set_text_content(Some(&format!("{:.2}", wave.amplitude)));
        required_element(&document, phase)?.set_text_content(Some(&report::degrees(Some(wave.phase))));
    }
    Ok(found)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the length list and the sliders back to what is on the page, after a change that could not be made.
fn restore_inputs(live: &Live) -> Result<(), String> {
    select(LENGTH_ID)?.set_value(&live.n.to_string());
    set_sliders(&live.waves)?;
    show_waves(&live.waves, live.n).map(drop)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn set_hidden(id: &str, hidden: bool) {
    let Ok(document) = document() else { return };
    if let Some(element) = document.get_element_by_id(id) {
        let _ = if hidden {
            element.set_attribute("hidden", "")
        } else {
            element.remove_attribute("hidden")
        };
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the HTML that depends on the state: the inspector's title, and the pair selector with its entries and its
/// choice. These are outside the scenes, so they are not replaced when a scene is.
fn render_html(live: &Live, completed: usize, pair: usize) -> Result<(), String> {
    let pairs = live.trace.stage_pairs(shown_stage(completed));
    let document = document()?;
    required_element(&document, TITLE_ID)?.set_text_content(Some(&words::inspector_title(
        completed,
        live.trace.stages(),
        pair + 1,
        pairs.len(),
    )));
    let list = select(PAIR_ID)?;
    let options: String = pairs
        .iter()
        .enumerate()
        .map(|(index, trace)| format!("<option value=\"{index}\">{}</option>", words::pair_option(index + 1, trace)))
        .collect();
    list.set_inner_html(&options);
    list.set_selected_index(i32::try_from(pair).map_err(stringify)?);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws both scenes for the state. Fails if a write fails, and writing it again repairs that.
fn render_scenes(live: &Live, completed: usize, pair: usize) -> Result<(), String> {
    live.overview.render(&live.trace, completed, pair)?;
    let selected = live
        .trace
        .stage_pairs(shown_stage(completed))
        .get(pair)
        .ok_or_else(|| format!("no pair {pair}"))?;
    live.inspector.show(selected)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws everything that shows the state. Not atomic: a failure can leave part of it written.
fn render_snapshot(live: &Live, completed: usize, pair: usize) -> Result<(), String> {
    render_scenes(live, completed, pair)?;
    render_html(live, completed, pair)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is said when the display has just reached `completed` stages with butterfly `pair` selected.
fn step_sentence(live: &Live, completed: usize, pair: usize) -> String {
    let stage = shown_stage(completed);
    let pairs = live.trace.stage_pairs(stage);
    let Some(selected) = pairs.get(pair) else { return String::new() };
    if completed == 0 {
        words::ready(live.n, pair + 1, selected)
    } else {
        words::stepped(completed, live.trace.stages(), pair + 1, pairs.len(), selected)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is said when butterfly `pair` is chosen.
fn pair_sentence(live: &Live, completed: usize, pair: usize) -> String {
    let pairs = live.trace.stage_pairs(shown_stage(completed));
    pairs.get(pair).map_or_else(String::new, |selected| {
        words::pair_chosen(completed, pair + 1, pairs.len(), selected)
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows what became of a request: the new state is said, or the error is shown, and the retry button comes and goes.
fn settle(outcome: &Outcome, said: String) {
    match outcome {
        Outcome::Committed => {
            clear_error(ERROR_ID);
            set_hidden(RETRY_ID, true);
            announce(STATUS_ID, &said);
        },
        Outcome::RolledBack(error) => report_error(ERROR_ID, &words::rolled_back(error)),
        Outcome::NeedsRepair(error) => {
            report_error(ERROR_ID, &words::needs_repair(error));
            set_hidden(RETRY_ID, false);
        },
        Outcome::Refused | Outcome::Resynchronised => {},
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The stepping toolbar moved. `generation` is the build whose toolbar this is.
fn on_step(generation: u64, transition: SelectionTransition) {
    let Some(live) = current(generation) else { return };
    let Some(mut cursor) = take_cursor() else { return };
    // "Restart" and "Previous" from the first stage both arrive as a move to unstarted. Either way the visited pairs
    // are forgotten, and the display returns to the default pair.
    if transition.to.is_none() {
        cursor.forget_choices();
    }
    let outcome = apply_transition(
        &mut cursor,
        &live.trace,
        transition.from,
        transition.to,
        &mut |completed, pair| render_snapshot(&live, completed, pair),
        &mut |completed| live.overview.set_driver(completed),
    );
    settle(&outcome, step_sentence(&live, cursor.completed(), cursor.pair()));
    put_cursor(cursor);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pair selector changed.
fn on_pair_changed() {
    let Some(live) = latest() else { return };
    let Some(mut cursor) = take_cursor() else { return };
    let chosen = select(PAIR_ID)
        .ok()
        .and_then(|list| usize::try_from(list.selected_index()).ok())
        .unwrap_or(cursor.pair());
    let outcome = apply_pair(&mut cursor, chosen, &mut |completed, pair| {
        render_snapshot(&live, completed, pair)
    });
    if outcome != Outcome::Committed {
        // The selector must not claim a pair that is not drawn.
        if let Ok(list) = select(PAIR_ID) {
            list.set_selected_index(i32::try_from(cursor.pair()).unwrap_or(0));
        }
    }
    settle(&outcome, pair_sentence(&live, cursor.completed(), cursor.pair()));
    put_cursor(cursor);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The retry button was pressed: draw the last good state again, and put the toolbar right.
fn on_retry() {
    let Some(live) = latest() else { return };
    let Some(mut cursor) = take_cursor() else { return };
    let result = retry(
        &mut cursor,
        &mut |completed, pair| render_snapshot(&live, completed, pair),
        &mut |completed| live.overview.set_driver(completed),
    );
    match result {
        Ok(()) => settle(&Outcome::Committed, step_sentence(&live, cursor.completed(), cursor.pair())),
        Err(error) => settle(&Outcome::NeedsRepair(error), String::new()),
    }
    put_cursor(cursor);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A slider is moving. The graph, the numbers and the findings follow it at once. The scenes wait until it is let go,
/// since drawing them takes longer than a slider takes to move.
fn on_wave_moving() {
    let result = chosen_size().and_then(|n| chosen_waves().and_then(|waves| show_waves(&waves, n)));
    match result {
        Ok(_) => clear_error(ERROR_ID),
        Err(error) => report_error(ERROR_ID, &format!("The waves could not be shown: {error}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A slider was let go, or the length changed.
fn on_input_changed() {
    let result = chosen_size().and_then(|n| chosen_waves().map(|waves| (n, waves)));
    let result = result.and_then(|(n, waves)| {
        let found = show_waves(&waves, n)?;
        rebuild(n, waves).map(|said| format!("{} {said}", report::announcement(&waves, &found)))
    });
    match result {
        Ok(said) => {
            clear_error(ERROR_ID);
            set_hidden(RETRY_ID, true);
            announce(STATUS_ID, &said);
        },
        Err(error) => {
            // The old scenes are still on the page and still work, so the controls go back to what they show.
            if let Some(live) = latest() {
                let _ = restore_inputs(&live);
            }
            report_error(
                ERROR_ID,
                &format!("The new transform could not be drawn, and the old one is kept: {error}"),
            );
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates a hidden `<svg>` named `id` in `stage`, for a scene to be drawn in while the old one stays on the page.
fn staging_svg(stage: &str, id: &str) -> Result<(), String> {
    let document = document()?;
    if let Some(stale) = document.get_element_by_id(id) {
        stale.remove();
    }
    ensure_svg_in(&document, stage, id, provisional())?;
    required_element(&document, id)?
        .set_attribute("style", STAGING_STYLE)
        .map_err(|e| format!("could not hide #{id}: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn discard_staging() {
    if let Ok(document) = document() {
        for id in [STAGING_OVERVIEW_ID, STAGING_INSPECTOR_ID] {
            if let Some(element) = document.get_element_by_id(id) {
                element.remove();
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts both replacement scenes on the page, in place of the old ones, or changes nothing.
///
/// Every step that can fail comes first, and touches only the replacements: each is made visible, which is the one write
/// that a browser can refuse. If one fails, the replacements already shown are hidden again, and the old scenes were never
/// touched. Only when both are shown does the swap begin, and the swap is made of steps that cannot fail: the old scene
/// gives up its id and is removed, and the replacement takes the id. So the page never holds a mixture of generations.
///
/// # Errors
///
/// Returns `Err` if a replacement is missing or cannot be shown. The page is then as it was, with the replacements still
/// hidden, for the caller to discard.
fn promote_all() -> Result<(), String> {
    let document = document()?;
    let mut shown: Vec<Element> = Vec::new();
    for (staging, _) in PROMOTIONS {
        let outcome = required_element(&document, staging).and_then(|svg: Element| {
            svg.remove_attribute("style")
                .map_err(|e| format!("could not show #{staging}: {e:?}"))?;
            shown.push(svg);
            Ok(())
        });
        if let Err(error) = outcome {
            for svg in &shown {
                let _ = svg.set_attribute("style", STAGING_STYLE);
            }
            return Err(error);
        }
    }
    for (svg, (_, id)) in shown.iter().zip(PROMOTIONS) {
        if let Some(old) = document.get_element_by_id(id) {
            old.remove();
        }
        svg.set_id(id);
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the transform of `n` samples of the sum of `waves`, and puts it on the page in place of the one that is there.
/// Returns what a screen reader should be told.
///
/// The new scenes are drawn out of sight first, and only a complete pair replaces the old. So a transform that cannot
/// be drawn leaves the old one in place and working. The zoom and pan of each scene are kept, and so is the place the
/// inspector's butterfly was dragged to, unless the new canvas has no room for it there. A toolbar button that had
/// focus is focused again afterwards.
///
/// # Errors
///
/// Returns `Err` if the transform cannot be worked out, or the page is missing a part, or any library call fails.
fn rebuild(n: usize, waves: [Wave; 3]) -> Result<String, String> {
    // Nothing on the page changes until the numbers are known to be good.
    if !SIZES.contains(&n) {
        return Err(format!("{n} samples are not offered"));
    }
    let fft = trace(&waves::samples(&waves, n)).map_err(|e| e.to_string())?;

    let previous = latest();
    let generation = RUNTIME.with_borrow(|runtime| runtime.generation) + 1;
    let overview_view = previous.as_ref().map(|live| live.overview.scene().view());
    let inspector_view = previous.as_ref().map(|live| live.inspector.scene().view());
    let origin = previous.as_ref().and_then(|live| live.inspector.frame_origin().ok());
    let focused = focus::focused_selector(OVERVIEW_ID);

    let drawn = draw(generation, n, waves, fft, overview_view, inspector_view, origin);
    let (live, cursor, moved) = match drawn {
        Ok(drawn) => drawn,
        Err(error) => {
            discard_staging();
            return Err(error);
        },
    };

    // The pair for the preview, and the first state, were written while the scenes were out of sight.
    if let Err(error) = promote_all() {
        discard_staging();
        return Err(error);
    }
    // The cursor was made with the pair that was drawn, so a repaint that fails, and a retry, go back to that pair.
    let pair = cursor.pair();
    let live = Rc::new(live);
    RUNTIME.with_borrow_mut(|runtime| {
        runtime.generation = generation;
        runtime.live = Some(live.clone());
        runtime.cursor = Some(cursor);
    });
    render_html(&live, 0, pair)?;
    if let Some(selector) = focused {
        focus::focus(&selector);
    }
    let mut said = step_sentence(&live, 0, pair);
    if moved {
        said.push_str(words::MOVED_INSIDE);
    }
    Ok(said)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws both scenes into their staging `<svg>` elements, and writes the unstarted state into them.
fn draw(
    generation: u64,
    n: usize,
    waves: [Wave; 3],
    fft: FftTrace,
    overview_view: Option<svg_dom_graph::scene::ViewTransform>,
    inspector_view: Option<svg_dom_graph::scene::ViewTransform>,
    origin: Option<svg_dom::root::utils::Point>,
) -> Result<(Live, Cursor, bool), String> {
    staging_svg(OVERVIEW_STAGE, STAGING_OVERVIEW_ID)?;
    staging_svg(INSPECTOR_STAGE, STAGING_INSPECTOR_ID)?;
    let overview = Overview::build(STAGING_OVERVIEW_ID, &fft, overview_view, move |_scene, _node, transition| {
        on_step(generation, transition);
    })?;
    let (inspector, moved) = Inspector::build(STAGING_INSPECTOR_ID, &fft, origin, inspector_view)?;
    let live = Live {
        generation,
        n,
        waves,
        trace: fft,
        overview,
        inspector,
    };
    let cursor = Cursor::unstarted(&live.trace);
    render_scenes(&live, 0, cursor.pair())?;
    Ok((live, cursor, moved))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn listen(id: &str, event: &str, mut handler: impl FnMut() + 'static) -> Result<(), String> {
    let element = required_element(&document()?, id)?;
    let listener: Listener = Closure::new(move |_event: web_sys::Event| handler());
    element
        .add_event_listener_with_callback(event, listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen to #{id}: {e:?}"))?;
    LISTENERS.with_borrow_mut(|listeners| listeners.push(listener));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the FFT demo: a forward, radix-2, decimation-in-time fast Fourier transform, one stage at a time.
///
/// - The **overview** has a column of lane nodes for the bit-reversed input and one for the result of each stage. Each
///   butterfly joins two lanes of one column to the same two lanes of the next by four connectors. The stepping toolbar
///   completes one stage at a time. A stage that has not been computed holds a faint zero, so that no result is shown
///   before it exists. The selected butterfly's lanes and connectors are emphasised.
/// - The **inspector** is one butterfly node. It shows the selected butterfly's two inputs, its twiddle factor, and its
///   product, sum and difference. Choosing another butterfly only gives the same nodes new numbers.
/// - The **graph** plots three sine waves and their sum, and the **sliders** set the amplitude and phase of each. The
///   sum is sampled `N` times and transformed, and a **table** shows the amplitude and phase the transform found beside
///   the ones that were set. Both follow a slider as it moves. The scenes are drawn again when it is let go.
/// - The **lists** above choose the number of samples and the butterfly.
///
/// Every value is worked out in plain Rust by [`algorithm::trace`], one whole block at a time. The library does no
/// arithmetic: it draws.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub(crate) fn build_fft_demo() -> Result<(), String> {
    let first = LISTENERS.with_borrow(Vec::is_empty);
    if first {
        set_sliders(&waves::initial())?;
    }
    let (n, waves) = (chosen_size()?, chosen_waves()?);
    let found = show_waves(&waves, n)?;
    let said = rebuild(n, waves)?;
    announce(STATUS_ID, &format!("{} {said}", report::announcement(&waves, &found)));
    if first {
        listen(LENGTH_ID, "change", on_input_changed)?;
        for [amplitude, phase, ..] in WAVE_CONTROLS {
            for id in [amplitude, phase] {
                listen(id, "input", on_wave_moving)?;
                listen(id, "change", on_input_changed)?;
            }
        }
        listen(PAIR_ID, "change", on_pair_changed)?;
        listen(RETRY_ID, "click", on_retry)?;
    }
    Ok(())
}
