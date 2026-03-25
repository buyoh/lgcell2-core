use std::io::{Write, stdout};
use std::time::Duration;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen};

/// 端末入出力を抽象化するトレイト。
pub trait Console {
    /// 端末サイズ (cols, rows) を返す。
    fn size(&self) -> Result<(u16, u16), String>;
    /// alternate screen + raw モードに入る。
    fn enter_alternate_screen(&mut self) -> Result<(), String>;
    /// alternate screen + raw モードから抜ける。
    fn leave_alternate_screen(&mut self) -> Result<(), String>;
    /// 画面バッファを書き込む。
    fn write_frame(&mut self, content: &str) -> Result<(), String>;
    /// キーイベントを待つ。timeout=None で無期限待機。
    fn poll_event(&self, timeout: Option<Duration>) -> Result<Option<KeyInput>, String>;
}

/// キー入力の抽象表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    Char(char),
    Up,
    Down,
    Left,
    Right,
}

/// crossterm を使った Console 実装。
#[derive(Debug, Default)]
pub struct CrosstermConsole;

impl CrosstermConsole {
    pub fn new() -> Self {
        Self
    }
}

impl Console for CrosstermConsole {
    fn size(&self) -> Result<(u16, u16), String> {
        terminal::size().map_err(|err| format!("failed to get terminal size: {err}"))
    }

    fn enter_alternate_screen(&mut self) -> Result<(), String> {
        terminal::enable_raw_mode().map_err(|err| format!("failed to enable raw mode: {err}"))?;

        let mut out = stdout();
        execute!(out, EnterAlternateScreen, cursor::Hide)
            .map_err(|err| format!("failed to enter alternate screen: {err}"))
    }

    fn leave_alternate_screen(&mut self) -> Result<(), String> {
        let mut out = stdout();
        execute!(out, cursor::Show, LeaveAlternateScreen)
            .map_err(|err| format!("failed to leave alternate screen: {err}"))?;

        terminal::disable_raw_mode().map_err(|err| format!("failed to disable raw mode: {err}"))
    }

    fn write_frame(&mut self, content: &str) -> Result<(), String> {
        let mut out = stdout();
        execute!(
            out,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All),
            crossterm::style::Print(content)
        )
        .map_err(|err| format!("failed to write frame: {err}"))?;

        out.flush()
            .map_err(|err| format!("failed to flush frame: {err}"))
    }

    fn poll_event(&self, timeout: Option<Duration>) -> Result<Option<KeyInput>, String> {
        if let Some(timeout) = timeout {
            let has_event = event::poll(timeout).map_err(|err| format!("poll failed: {err}"))?;
            if !has_event {
                return Ok(None);
            }
            return read_key_event();
        }

        loop {
            if let Some(key) = read_key_event()? {
                return Ok(Some(key));
            }
        }
    }
}

fn read_key_event() -> Result<Option<KeyInput>, String> {
    let event = event::read().map_err(|err| format!("read event failed: {err}"))?;
    let Event::Key(key_event) = event else {
        return Ok(None);
    };

    let mapped = match key_event.code {
        KeyCode::Char(c) => Some(KeyInput::Char(c)),
        KeyCode::Up => Some(KeyInput::Up),
        KeyCode::Down => Some(KeyInput::Down),
        KeyCode::Left => Some(KeyInput::Left),
        KeyCode::Right => Some(KeyInput::Right),
        _ => None,
    };

    Ok(mapped)
}
