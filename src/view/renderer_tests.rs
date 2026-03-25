use std::collections::BTreeSet;

use crate::circuit::{Circuit, Pos};
use crate::simulation::SimState;
use crate::view::renderer::ViewRenderer;

fn make_state(cells: &[(i32, i32)], on_cells: &[(i32, i32)]) -> SimState {
    let mut set = BTreeSet::new();
    for (x, y) in cells {
        set.insert(Pos::new(*x, *y));
    }

    let circuit = Circuit::new(set, vec![]).expect("circuit must be valid");
    let mut state = SimState::from_circuit(&circuit);

    for (x, y) in on_cells {
        state
            .set(Pos::new(*x, *y), true)
            .expect("cell should exist in state");
    }

    state
}

#[test]
fn render_grid_maps_symbols() {
    let state = make_state(&[(0, 0), (1, 0), (0, 1)], &[(0, 0), (0, 1)]);
    let renderer = ViewRenderer::new(0, 0);

    let grid = renderer.render_grid(&state, 2, 2);

    assert_eq!(grid, "#_\r\n#.");
}

#[test]
fn render_grid_respects_viewport_offset() {
    let state = make_state(&[(5, 5), (6, 5)], &[(6, 5)]);
    let renderer = ViewRenderer::new(5, 5);

    let grid = renderer.render_grid(&state, 2, 1);

    assert_eq!(grid, "_#");
}

#[test]
fn render_status_bar_pads_to_terminal_width() {
    let renderer = ViewRenderer::new(0, 0);

    let status = renderer.render_status_bar(42, false, 10, 4, 60);

    assert_eq!(status.chars().count(), 60);
    assert!(status.contains("tick:42 | running"));
    assert!(status.contains("(q:quit space:pause arrows:scroll)"));
}

#[test]
fn scroll_updates_viewport() {
    let mut renderer = ViewRenderer::new(1, 2);

    renderer.scroll(-3, 4);

    assert_eq!(renderer.viewport(), (-2, 6));
}
