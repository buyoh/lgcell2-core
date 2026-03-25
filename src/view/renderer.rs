use crate::circuit::Pos;
use crate::simulation::SimState;

const HELP_TEXT: &str = "(q:quit space:pause arrows:scroll)";

/// ビューモードのフレーム文字列を生成するレンダラー。
#[derive(Debug, Clone)]
pub struct ViewRenderer {
    viewport_x: i32,
    viewport_y: i32,
}

impl ViewRenderer {
    pub fn new(viewport_x: i32, viewport_y: i32) -> Self {
        Self {
            viewport_x,
            viewport_y,
        }
    }

    /// cols x rows のグリッド領域を生成する。
    pub fn render_grid(&self, state: &SimState, cols: u16, rows: u16) -> String {
        let mut output = String::with_capacity((cols as usize + 1) * rows as usize);

        for row in 0..rows {
            for col in 0..cols {
                let pos = Pos::new(self.viewport_x + col as i32, self.viewport_y + row as i32);
                let ch = match state.get(pos) {
                    Some(true) => '#',
                    Some(false) => '_',
                    None => '.',
                };
                output.push(ch);
            }
            if row + 1 < rows {
                output.push('\n');
            }
        }

        output
    }

    /// ステータスバー文字列を生成する。
    pub fn render_status_bar(
        &self,
        tick: u64,
        paused: bool,
        grid_cols: u16,
        grid_rows: u16,
        total_width: u16,
    ) -> String {
        let state = if paused { "paused" } else { "running" };

        let x1 = self.viewport_x;
        let y1 = self.viewport_y;
        let x2 = x1 + i32::from(grid_cols.saturating_sub(1));
        let y2 = y1 + i32::from(grid_rows.saturating_sub(1));

        let left = format!("tick:{tick} | {state} | ({x1},{y1})-({x2},{y2})");
        fit_status_line(&left, HELP_TEXT, total_width as usize)
    }

    /// グリッド + ステータスバーを結合したフレーム文字列を生成する。
    pub fn render_frame(
        &self,
        state: &SimState,
        tick: u64,
        paused: bool,
        cols: u16,
        rows: u16,
    ) -> String {
        if rows == 0 {
            return String::new();
        }

        let grid_rows = rows.saturating_sub(1);
        let grid = self.render_grid(state, cols, grid_rows);
        let status = self.render_status_bar(tick, paused, cols, grid_rows, cols);

        if grid_rows == 0 {
            status
        } else {
            format!("{grid}\n{status}")
        }
    }

    pub fn scroll(&mut self, dx: i32, dy: i32) {
        self.viewport_x += dx;
        self.viewport_y += dy;
    }

    pub fn viewport(&self) -> (i32, i32) {
        (self.viewport_x, self.viewport_y)
    }
}

fn fit_status_line(left: &str, right: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let left_count = left.chars().count();
    let right_count = right.chars().count();

    if left_count + 1 + right_count <= width {
        let spaces = width - left_count - right_count;
        return format!("{left}{}{}", " ".repeat(spaces), right);
    }

    if right_count >= width {
        return right.chars().take(width).collect();
    }

    let max_left = width - right_count - 1;
    let truncated_left: String = left.chars().take(max_left).collect();
    let spaces = width - truncated_left.chars().count() - right_count;
    format!("{truncated_left}{}{}", " ".repeat(spaces), right)
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod renderer_tests;
