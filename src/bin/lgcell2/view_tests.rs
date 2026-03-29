#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeSet, VecDeque};
    use std::rc::Rc;
    use std::time::Duration;

    use lgcell2_core::circuit::{Circuit, Pos};
    use lgcell2_core::platform::console::{Console, KeyInput};

    use crate::view::run_view_loop_with_config;

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
        assert!(borrowed.frames.iter().any(|frame| frame.contains("tick:0")));
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
        let result = crate::view::run_view_loop(console, single_cell_circuit());
        assert!(result.is_ok());
    }
}
