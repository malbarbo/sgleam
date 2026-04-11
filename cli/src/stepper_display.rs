use std::io::{self, Read, Write};

use crossterm::{
    cursor, execute, queue,
    style::{self, Stylize},
    terminal::{self, ClearType},
};
use engine::substitution::SubstitutionStep;

use crate::repl_reader::highlight_gleam;

pub fn display_stepper(steps: &[SubstitutionStep]) -> Result<(), io::Error> {
    if steps.is_empty() {
        println!("No steps to display.");
        return Ok(());
    }

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All)
    )?;

    let mut current = 0usize;
    let res = run_stepper(&mut stdout, steps, &mut current);

    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    res
}

fn run_stepper(
    stdout: &mut io::Stdout,
    steps: &[SubstitutionStep],
    current: &mut usize,
) -> Result<(), io::Error> {
    loop {
        render_frame(stdout, steps, *current)?;
        stdout.flush()?;

        let key = read_key()?;
        match key {
            KeyCode::Right | KeyCode::Down | KeyCode::Char('l') | KeyCode::Char('j') => {
                if *current < steps.len() {
                    *current += 1;
                }
            }
            KeyCode::Left | KeyCode::Up | KeyCode::Char('h') | KeyCode::Char('k') => {
                if *current > 0 {
                    *current -= 1;
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(());
            }
            KeyCode::CtrlC => {
                return Ok(());
            }
            _ => {}
        }
    }
}

enum KeyCode {
    Up,
    Down,
    Left,
    Right,
    Char(char),
    Esc,
    CtrlC,
    Other,
}

fn read_key() -> Result<KeyCode, io::Error> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 3];
    let n = stdin.read(&mut buf)?;

    if n == 0 {
        return Ok(KeyCode::Other);
    }

    match buf[0] {
        27 => {
            if n == 1 {
                Ok(KeyCode::Esc)
            } else if n >= 3 && buf[1] == 91 {
                match buf[2] {
                    65 => Ok(KeyCode::Up),
                    66 => Ok(KeyCode::Down),
                    67 => Ok(KeyCode::Right),
                    68 => Ok(KeyCode::Left),
                    _ => Ok(KeyCode::Other),
                }
            } else {
                Ok(KeyCode::Other)
            }
        }
        3 => Ok(KeyCode::CtrlC),
        c => Ok(KeyCode::Char(c as char)),
    }
}

fn render_frame(
    stdout: &mut io::Stdout,
    steps: &[SubstitutionStep],
    current: usize,
) -> Result<(), io::Error> {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let cols = cols as usize;
    let rows = rows as usize;
    let total_steps = steps.len();

    queue!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All)
    )?;

    let left_step = if current > 0 {
        steps.get(current - 1)
    } else {
        None
    };
    let right_step = if current < total_steps {
        steps.get(current)
    } else {
        None
    };

    let title_str = if current < total_steps {
        format!(" Stepper - Step {}/{} ", current + 1, total_steps)
    } else {
        " Stepper - Finished ".to_string()
    };
    let help_str = " q: quit, arrows: navigate ";

    let available_width = cols.saturating_sub(7);
    let col_left_w = available_width / 2;
    let col_right_w = available_width - col_left_w;

    let left_fill = col_left_w + 1;
    let right_fill = col_right_w + 1;

    let title_trunc = truncate_str(&title_str, left_fill);
    let help_trunc = truncate_str(help_str, right_fill);

    let title_len = title_trunc.chars().count();
    let left_pad = left_fill.saturating_sub(title_len);

    let help_len = help_trunc.chars().count();
    let right_pad = right_fill.saturating_sub(help_len);

    queue!(
        stdout,
        style::PrintStyledContent("╭─".cyan()),
        style::PrintStyledContent(title_trunc.yellow().bold()),
        style::PrintStyledContent("─".repeat(left_pad).cyan()),
        style::PrintStyledContent("┬".cyan()),
        style::PrintStyledContent("─".repeat(right_pad).cyan()),
        style::PrintStyledContent(help_trunc.grey()),
        style::PrintStyledContent("─╮\r\n".cyan()),
    )?;

    let transition_note = right_step
        .and_then(|s| s.note.as_deref())
        .unwrap_or("")
        .trim();

    let left_title = if left_step.is_some() {
        if transition_note.is_empty() {
            format!("Step {}", current)
        } else {
            format!("Step {} ({})", current, transition_note)
        }
    } else {
        "".to_string()
    };

    let right_title = if right_step.is_some() {
        format!("Step {}", current + 1)
    } else {
        "".to_string()
    };

    queue!(
        stdout,
        style::PrintStyledContent("│ ".cyan()),
        style::PrintStyledContent(
            format!(
                "{:<width$}",
                truncate_str(&left_title, col_left_w),
                width = col_left_w
            )
            .bold()
        ),
        style::PrintStyledContent(" │ ".cyan()),
        style::PrintStyledContent(
            format!(
                "{:<width$}",
                truncate_str(&right_title, col_right_w),
                width = col_right_w
            )
            .bold()
        ),
        style::PrintStyledContent(" │\r\n".cyan()),
    )?;

    queue!(
        stdout,
        style::PrintStyledContent("├─".cyan()),
        style::PrintStyledContent("─".repeat(col_left_w).cyan()),
        style::PrintStyledContent("─┼─".cyan()),
        style::PrintStyledContent("─".repeat(col_right_w).cyan()),
        style::PrintStyledContent("─┤\r\n".cyan())
    )?;

    let left_lines: Vec<String> = left_step
        .map(|s| s.formatted.lines().map(highlight_gleam).collect())
        .unwrap_or_default();
    let right_lines: Vec<String> = right_step
        .map(|s| s.formatted.lines().map(highlight_gleam).collect())
        .unwrap_or_default();

    let context_lines: Vec<String> = right_step
        .and_then(|s| s.context.as_deref())
        .map(|c| c.lines().map(highlight_gleam).collect())
        .unwrap_or_default();

    let mut context_rows = context_lines.len();
    let mut content_rows = rows.saturating_sub(5);

    if context_rows > 0 {
        if content_rows > context_rows + 4 {
            content_rows -= context_rows + 1;
        } else {
            context_rows = 0;
        }
    }

    for i in 0..content_rows {
        queue!(stdout, style::PrintStyledContent("│ ".cyan()))?;

        if i < left_lines.len() {
            print_highlighted_bounded(stdout, &left_lines[i], col_left_w)?;
        } else {
            queue!(stdout, style::Print(" ".repeat(col_left_w)))?;
        }

        queue!(stdout, style::PrintStyledContent(" │ ".cyan()))?;

        if i < right_lines.len() {
            print_highlighted_bounded(stdout, &right_lines[i], col_right_w)?;
        } else {
            queue!(stdout, style::Print(" ".repeat(col_right_w)))?;
        }

        queue!(stdout, style::PrintStyledContent(" │\r\n".cyan()))?;
    }

    if context_rows > 0 {
        queue!(
            stdout,
            style::PrintStyledContent("├─".cyan()),
            style::PrintStyledContent("─".repeat(col_left_w).cyan()),
            style::PrintStyledContent("─┴─".cyan()),
            style::PrintStyledContent("─".repeat(col_right_w).cyan()),
            style::PrintStyledContent("─┤\r\n".cyan())
        )?;

        for i in 0..context_rows {
            queue!(stdout, style::PrintStyledContent("│ ".cyan()))?;
            if let Some(line) = context_lines.get(i) {
                print_highlighted_bounded(stdout, line, cols.saturating_sub(4))?;
            } else {
                queue!(stdout, style::Print(" ".repeat(cols.saturating_sub(4))))?;
            }
            queue!(stdout, style::PrintStyledContent(" │\r\n".cyan()))?;
        }

        queue!(
            stdout,
            style::PrintStyledContent("╰─".cyan()),
            style::PrintStyledContent("─".repeat(cols.saturating_sub(4)).cyan()),
            style::PrintStyledContent("─╯\r\n".cyan())
        )?;
    } else {
        queue!(
            stdout,
            style::PrintStyledContent("╰─".cyan()),
            style::PrintStyledContent("─".repeat(col_left_w).cyan()),
            style::PrintStyledContent("─┴─".cyan()),
            style::PrintStyledContent("─".repeat(col_right_w).cyan()),
            style::PrintStyledContent("─╯\r\n".cyan())
        )?;
    }

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
    format!("{}...", truncated)
}

fn print_highlighted_bounded(
    stdout: &mut io::Stdout,
    highlighted: &str,
    width: usize,
) -> Result<(), io::Error> {
    let mut visible_count = 0;
    let mut i = 0;
    let bytes = highlighted.as_bytes();
    let mut result = String::new();

    while i < bytes.len() && visible_count < width {
        if bytes[i] == b'\x1b' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            result.push_str(&highlighted[start..i]);
        } else {
            let s = &highlighted[i..];
            if let Some(c) = s.chars().next() {
                if visible_count < width {
                    result.push(c);
                    visible_count += 1;
                    i += c.len_utf8();
                } else {
                    result.push('.');
                    visible_count += 1;
                    break;
                }
            } else {
                break;
            }
        }
    }

    result.push_str("\x1b[0m");

    queue!(stdout, style::Print(result))?;
    if visible_count < width {
        queue!(stdout, style::Print(" ".repeat(width - visible_count)))?;
    }
    Ok(())
}
