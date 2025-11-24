use crate::cli::{LocksArgs, ResolveArgs};
use crate::error::Error;
use crate::humanize;
use crate::locks::http::{fetch_detailed_poisoned_locks, fetch_locks_raw, unlock_lock};
use crate::locks::types::{LockType, PoisonedLock};
use crate::tui::utils::{centered_rect, get_longest_len};
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{event, execute};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, HighlightSpacing, Row, Table, TableState};
use std::io;

impl<'a> From<&'a PoisonedLock> for Row<'a> {
    fn from(value: &'a PoisonedLock) -> Self {
        let poisoned_by = value.get_poisoned_by();
        let task_name = poisoned_by.display_name.as_str();
        let name = value.name();
        let ty = value.get_type();
        let uri = poisoned_by.flake_ref.flake.as_str();
        Row::new(vec![name, ty, task_name, uri])
    }
}

pub fn show_locks(locks_args: &LocksArgs) -> Result<(), Error> {
    if locks_args.ctx.humanize {
        humanize::ensure_jless("lock")?;
    }
    if locks_args.active && locks_args.poisoned {
        return Err(Error::Custom(
            "Cannot use active and poisoned lock type at the same time.",
        ));
    }

    let locks_json = fetch_locks_raw(&locks_args.ctx, LockType::from(locks_args), false)?;

    humanize::handle_user_response(&locks_args.ctx, &locks_json)?;
    Ok(())
}

pub fn resolve_lock(resolve_args: &ResolveArgs) -> Result<(), Error> {
    let mut locks = fetch_detailed_poisoned_locks(&resolve_args.ctx)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TableState::default();
    state.select(Some(0));
    let mut selected_task: Option<usize> = None;
    let mut selected_button: bool = false;

    let mut should_quit = false;
    while !should_quit {
        should_quit = handle_events(
            &mut state,
            locks.len(),
            &mut selected_task,
            &mut selected_button,
            resolve_args,
            &mut locks,
        )?;
        terminal.draw(|f| ui(f, &locks, &mut state, &selected_task, &mut selected_button))?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn unlock_and_refresh(
    resolve_args: &ResolveArgs,
    locks: &mut Vec<PoisonedLock>,
    selected_task: &mut Option<usize>,
) -> Result<(), Error> {
    if let Some(task_idx) = selected_task {
        unlock_lock(&resolve_args.ctx, &locks[*task_idx])?;
        *locks = fetch_detailed_poisoned_locks(&resolve_args.ctx)?;
        *selected_task = None;
    }
    Ok(())
}

fn handle_popup(
    selected_task: &mut Option<usize>,
    selected_button: &mut bool,
    key: &KeyEvent,
    args: &ResolveArgs,
    locks: &mut Vec<PoisonedLock>,
) -> Result<(), Error> {
    if key.code == KeyCode::Left || key.code == KeyCode::Char('y') {
        *selected_button = true;
    } else if key.code == KeyCode::Right || key.code == KeyCode::Char('n') {
        *selected_button = false;
    }

    if key.code == KeyCode::Char('y') || (key.code == KeyCode::Enter && *selected_button) {
        unlock_and_refresh(args, locks, selected_task)?;
    } else if key.code == KeyCode::Char('n') || (key.code == KeyCode::Enter && !*selected_button) {
        *selected_task = None;
    }

    Ok(())
}

fn handle_events(
    state: &mut TableState,
    lock_amount: usize,
    selected_task: &mut Option<usize>,
    selected_button: &mut bool,
    args: &ResolveArgs,
    locks: &mut Vec<PoisonedLock>,
) -> Result<bool, Error> {
    if !event::poll(std::time::Duration::from_millis(50))? {
        return Ok(false);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != event::KeyEventKind::Press {
            return Ok(false);
        }

        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
            if selected_task.is_some() {
                *selected_task = None;
                return Ok(false);
            }
            return Ok(true);
        }

        match selected_task {
            None => handle_task_list(state, lock_amount, selected_task, &key),
            Some(_) => handle_popup(selected_task, selected_button, &key, args, locks)?,
        }
    }

    Ok(false)
}

fn handle_task_list(
    state: &mut TableState,
    lock_amount: usize,
    selected_task: &mut Option<usize>,
    key: &KeyEvent,
) {
    if let Some(cur) = state.selected_mut() {
        handle_task_list_input(lock_amount, selected_task, key, cur);
    };
}

fn handle_task_list_input(
    lock_amount: usize,
    selected_task: &mut Option<usize>,
    key: &KeyEvent,
    cur: &mut usize,
) {
    if (key.code == KeyCode::Up || key.code == KeyCode::Char('k')) && *cur > 0 {
        *cur = cur.saturating_sub(1);
    } else if (key.code == KeyCode::Down || key.code == KeyCode::Char('j'))
        && *cur < lock_amount.saturating_sub(1)
    {
        *cur += 1;
    } else if key.code == KeyCode::Enter {
        *selected_task = Some(*cur);
    }
}

// This will not make the table equally spaced, but instead use minimal space.
#[allow(dead_code)]
fn minimal_widths(locks: &[PoisonedLock]) -> [Constraint; 4] {
    [
        Constraint::Max(get_longest_len(locks.iter().map(|l| l.name()))),
        Constraint::Max(5),
        Constraint::Max(
            get_longest_len(
                locks
                    .iter()
                    .map(|l| l.get_poisoned_by().display_name.as_str()),
            )
            .max("Failed Task Name".len() as u16),
        ),
        Constraint::Min(
            get_longest_len(
                locks
                    .iter()
                    .map(|l| l.get_poisoned_by().flake_ref.flake.as_str()),
            )
            .max("Task Flake URI".len() as u16),
        ),
    ]
}

fn draw_task_picker(f: &mut Frame, locks: &[PoisonedLock], state: &mut TableState) {
    let rows: Vec<Row> = locks.iter().map(|l| l.into()).collect();

    // let widths = minimal_widths(locks);
    let widths = [];
    let table = Table::new(rows, &widths)
        .block(
            Block::default()
                .title("Manually Resolve Locks")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .header(
            Row::new(vec!["Name", "Type", "Failed Task Name", "Task Flake URI"])
                .set_style(Style::default().bold().italic()),
        )
        .highlight_symbol(">>")
        .highlight_style(Style::default().fg(Color::Green).italic())
        .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(table, f.size(), state);
}

fn draw_confirm_clear(
    f: &mut Frame,
    locks: &[PoisonedLock],
    selected: usize,
    button_select: &mut bool,
) {
    let lock = locks.get(selected);
    if lock.is_none() {
        return;
    }
    let lock = match lock {
        Some(lock) => lock,
        None => return,
    };
    draw_centered_popup(
        f,
        &format!("Do you really want to clear the lock {}?", lock.name()),
        button_select,
    );
}

pub fn ui(
    f: &mut Frame,
    locks: &[PoisonedLock],
    state: &mut TableState,
    selected_task: &Option<usize>,
    button_select: &mut bool,
) {
    draw_task_picker(f, locks, state);
    if let Some(selected) = selected_task {
        draw_confirm_clear(f, locks, *selected, button_select);
    }
}

fn draw_centered_popup(f: &mut Frame, title: &str, button_select: &mut bool) {
    let mut yes = Text::from("Yes").bold().alignment(Alignment::Center);
    let mut no = Text::from("No").bold().alignment(Alignment::Center);
    if *button_select {
        yes = yes.fg(Color::Green);
    } else {
        no = no.fg(Color::Green);
    }

    let container = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center);
    let centered_rect = centered_rect(60, 20, f.size());
    let half_y = centered_rect.height / 2;
    let half_x = centered_rect.width / 2;
    let left_side = Rect::new(centered_rect.x, centered_rect.y + half_y, half_x, 1);
    let right_side = Rect::new(
        centered_rect.x + half_x,
        centered_rect.y + half_y,
        half_x,
        1,
    );

    f.render_widget(container, centered_rect);
    f.render_widget(yes, left_side);
    f.render_widget(no, right_side);
}
