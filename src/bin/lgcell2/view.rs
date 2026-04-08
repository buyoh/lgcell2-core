use std::time::{Duration, Instant};

use lgcell2_core::circuit::Circuit;
use lgcell2_core::platform::console::{Console, CrosstermConsole, KeyInput};
use lgcell2_core::simulation::{Simulator, SimulatorSimple};
use lgcell2_core::view::ViewRenderer;

const TICK_INTERVAL: Duration = Duration::from_millis(200);

/// ビューモードのエントリポイント。
pub fn run_view_mode(circuit: Circuit) -> Result<(), String> {
    if !circuit.modules().is_empty() {
        return Err("view mode does not support circuits with sub-circuit modules".to_string());
    }
    let console = CrosstermConsole::new();
    run_view_loop(console, circuit)
}

fn run_view_loop<C: Console>(console: C, circuit: Circuit) -> Result<(), String> {
    run_view_loop_with_config(console, circuit, TICK_INTERVAL, None)
}

fn run_view_loop_with_config<C: Console>(
    mut console: C,
    circuit: Circuit,
    tick_interval: Duration,
    max_iterations: Option<usize>,
) -> Result<(), String> {
    console.enter_alternate_screen()?;

    let run_result = (|| {
        let (x, y) = initial_viewport(&circuit);
        let mut simulator = SimulatorSimple::new(circuit);
        let mut renderer = ViewRenderer::new(x, y);
        let mut paused = false;
        let mut tick_started_at = Instant::now();
        let mut iterations = 0usize;

        render_once(&mut console, &renderer, &simulator, paused)?;

        loop {
            if let Some(limit) = max_iterations {
                if iterations >= limit {
                    break;
                }
            }
            iterations += 1;

            let timeout = if paused {
                None
            } else {
                Some(tick_interval.saturating_sub(tick_started_at.elapsed()))
            };

            if let Some(key) = console.poll_event(timeout)? {
                match key {
                    KeyInput::Char('q') => break,
                    KeyInput::Char(' ') => paused = !paused,
                    KeyInput::Up => renderer.scroll(0, -1),
                    KeyInput::Down => renderer.scroll(0, 1),
                    KeyInput::Left => renderer.scroll(-1, 0),
                    KeyInput::Right => renderer.scroll(1, 0),
                    KeyInput::Char(_) => {}
                }
            }

            if !paused && tick_started_at.elapsed() >= tick_interval {
                simulator.tick();
                tick_started_at = Instant::now();
            }

            render_once(&mut console, &renderer, &simulator, paused)?;
        }

        Ok(())
    })();

    let leave_result = console.leave_alternate_screen();

    match (run_result, leave_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(leave_err)) => Err(leave_err),
        (Err(run_err), Err(leave_err)) => Err(format!(
            "view loop failed: {run_err}; and terminal restore failed: {leave_err}"
        )),
    }
}

fn render_once<C: Console>(
    console: &mut C,
    renderer: &ViewRenderer,
    simulator: &SimulatorSimple,
    paused: bool,
) -> Result<(), String> {
    let output = simulator.last_output();
    let (cols, rows) = console.size()?;
    let frame = renderer.render_frame(&output.cells, output.tick, paused, cols, rows);
    console.write_frame(&frame)
}

fn initial_viewport(circuit: &Circuit) -> (i32, i32) {
    let mut cells = circuit.cells().iter();
    let Some(first) = cells.next() else {
        return (0, 0);
    };

    let mut min_x = first.x;
    let mut min_y = first.y;

    for pos in cells {
        if pos.x < min_x {
            min_x = pos.x;
        }
        if pos.y < min_y {
            min_y = pos.y;
        }
    }

    (min_x, min_y)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod view_tests;
