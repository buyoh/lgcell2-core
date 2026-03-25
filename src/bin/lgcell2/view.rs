use std::time::{Duration, Instant};

use lgcell2_core::circuit::Circuit;
use lgcell2_core::platform::console::{Console, CrosstermConsole, KeyInput};
use lgcell2_core::simulation::Simulator;
use lgcell2_core::view::ViewRenderer;

const TICK_INTERVAL: Duration = Duration::from_millis(200);

/// ビューモードのエントリポイント。
pub fn run_view_mode(circuit: Circuit) -> Result<(), String> {
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
        let mut simulator = Simulator::new(circuit);
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
    simulator: &Simulator,
    paused: bool,
) -> Result<(), String> {
    let (cols, rows) = console.size()?;
    let frame = renderer.render_frame(
        simulator.state(),
        simulator.current_tick(),
        paused,
        cols,
        rows,
    );
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
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeSet, VecDeque};
    use std::rc::Rc;
    use std::time::Duration;

    use lgcell2_core::circuit::{Circuit, Pos};
    use lgcell2_core::platform::console::{Console, KeyInput};

    use super::run_view_loop_with_config;

    #[derive(Debug, Default)]
    struct StubData {
        entered: bool,
        left: bool,
        events: VecDeque<Option<KeyInput>>,
        frames: Vec<String>,
    }

    #[derive(Debug, Clone)]
    struct StubConsole {
        size: (u16, u16),
        data: Rc<RefCell<StubData>>,
    }

    impl StubConsole {
        fn new(size: (u16, u16), events: Vec<Option<KeyInput>>) -> Self {
            Self {
                size,
                data: Rc::new(RefCell::new(StubData {
                    entered: false,
                    left: false,
                    events: events.into(),
                    frames: Vec::new(),
                })),
            }
        }
    }

    impl Console for StubConsole {
        fn size(&self) -> Result<(u16, u16), String> {
            Ok(self.size)
        }

        fn enter_alternate_screen(&mut self) -> Result<(), String> {
            self.data.borrow_mut().entered = true;
            Ok(())
        }

        fn leave_alternate_screen(&mut self) -> Result<(), String> {
            self.data.borrow_mut().left = true;
            Ok(())
        }

        fn write_frame(&mut self, content: &str) -> Result<(), String> {
            self.data.borrow_mut().frames.push(content.to_string());
            Ok(())
        }

        fn poll_event(&self, _timeout: Option<Duration>) -> Result<Option<KeyInput>, String> {
            let next = self.data.borrow_mut().events.pop_front().unwrap_or(None);
            Ok(next)
        }
    }

    fn single_cell_circuit() -> Circuit {
        let mut cells = BTreeSet::new();
        cells.insert(Pos::new(0, 0));
        Circuit::new(cells, vec![]).expect("circuit must be valid")
    }

    #[test]
    fn q_key_exits_and_restores_terminal() {
        let console = StubConsole::new((20, 4), vec![Some(KeyInput::Char('q'))]);
        let data = Rc::clone(&console.data);

        let result = run_view_loop_with_config(
            console,
            single_cell_circuit(),
            Duration::from_millis(200),
            Some(10),
        );

        assert!(result.is_ok());
        let borrowed = data.borrow();
        assert!(borrowed.entered);
        assert!(borrowed.left);
        assert!(!borrowed.frames.is_empty());
    }

    #[test]
    fn space_toggles_pause_in_status_bar() {
        let console = StubConsole::new(
            (60, 4),
            vec![Some(KeyInput::Char(' ')), Some(KeyInput::Char('q'))],
        );
        let data = Rc::clone(&console.data);

        let result = run_view_loop_with_config(
            console,
            single_cell_circuit(),
            Duration::from_millis(200),
            Some(10),
        );

        assert!(result.is_ok());
        let borrowed = data.borrow();
        let last_frame = borrowed.frames.last().expect("frame should exist");
        assert!(last_frame.contains("paused"));
    }

    #[test]
    fn arrow_key_scrolls_viewport_coordinates() {
        let console = StubConsole::new(
            (60, 4),
            vec![
                Some(KeyInput::Right),
                Some(KeyInput::Down),
                Some(KeyInput::Char('q')),
            ],
        );
        let data = Rc::clone(&console.data);

        let result = run_view_loop_with_config(
            console,
            single_cell_circuit(),
            Duration::from_millis(200),
            Some(10),
        );

        assert!(result.is_ok());
        let borrowed = data.borrow();
        let last_frame = borrowed.frames.last().expect("frame should exist");
        assert!(last_frame.contains("(1,1)-"));
    }

    #[test]
    fn auto_tick_progresses_when_not_paused() {
        let console = StubConsole::new((60, 4), vec![None, Some(KeyInput::Char('q'))]);
        let data = Rc::clone(&console.data);

        let result =
            run_view_loop_with_config(console, single_cell_circuit(), Duration::ZERO, Some(10));

        assert!(result.is_ok());
        let borrowed = data.borrow();
        assert!(borrowed.frames.iter().any(|frame| frame.contains("tick:1")));
    }

    #[test]
    fn leave_is_called_even_if_write_fails() {
        #[derive(Debug, Clone)]
        struct FailingWriteConsole {
            data: Rc<RefCell<StubData>>,
        }

        impl Console for FailingWriteConsole {
            fn size(&self) -> Result<(u16, u16), String> {
                Ok((20, 4))
            }

            fn enter_alternate_screen(&mut self) -> Result<(), String> {
                self.data.borrow_mut().entered = true;
                Ok(())
            }

            fn leave_alternate_screen(&mut self) -> Result<(), String> {
                self.data.borrow_mut().left = true;
                Ok(())
            }

            fn write_frame(&mut self, _content: &str) -> Result<(), String> {
                Err("write failed".to_string())
            }

            fn poll_event(&self, _timeout: Option<Duration>) -> Result<Option<KeyInput>, String> {
                Ok(Some(KeyInput::Char('q')))
            }
        }

        let data = Rc::new(RefCell::new(StubData::default()));
        let console = FailingWriteConsole {
            data: Rc::clone(&data),
        };

        let result = run_view_loop_with_config(
            console,
            single_cell_circuit(),
            Duration::from_millis(200),
            Some(10),
        );

        assert!(result.is_err());
        assert!(data.borrow().left);
    }

    #[test]
    fn run_view_loop_wrapper_is_used() {
        let console = StubConsole::new((20, 4), vec![Some(KeyInput::Char('q'))]);
        let result = super::run_view_loop(console, single_cell_circuit());
        assert!(result.is_ok());
    }
}
