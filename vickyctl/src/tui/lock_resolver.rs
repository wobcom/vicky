use std::io;

use crossterm::{event, execute};
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::{AppContext, ResolveArgs};
use crate::error::Error;
use crate::http_client::prepare_client;
use crate::locks::PoisonedLock;
use crate::tui::popup::draw_centered_popup;

pub fn resolve_lock(resolve_args: &ResolveArgs) -> Result<(), Error> {
    let mut locks = crate::locks::fetch_detailed_poisoned_locks(&resolve_args.ctx)?;

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
        *locks = crate::locks::fetch_detailed_poisoned_locks(&resolve_args.ctx)?;
        *selected_task = None;
    }
    Ok(())
}

fn unlock_lock(ctx: &AppContext, lock_to_clear: &PoisonedLock) -> Result<(), Error> {
    let client = prepare_client(ctx)?;
    let request = client
        .patch(format!(
            "{}/api/v1/locks/unlock/{}",
            ctx.vicky_url,
            lock_to_clear.id()
        ))
        .build()?;
    client.execute(request)?.error_for_status()?;
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
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
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
    match state.selected_mut() {
        None => (),
        Some(cur) => {
            if (key.code == KeyCode::Up || key.code == KeyCode::Char('k')) && *cur > 0 {
                *cur -= 1;
            } else if (key.code == KeyCode::Down || key.code == KeyCode::Char('j'))
                && *cur < lock_amount - 1
            {
                *cur += 1;
            } else if key.code == KeyCode::Enter {
                *selected_task = Some(*cur);
            }
        }
    };
}

#[allow(dead_code)]
fn get_longest_len<'a, T>(str_iter: T) -> u16
where
    T: Iterator<Item = &'a str>,
{
    str_iter
        .map(|l| l.len())
        .max()
        .map_or(0, |len| u16::try_from(len).unwrap_or(u16::MAX))
}

// This will not make the table equally spaced, but instead use minimal space.
#[allow(dead_code)]
fn minimal_widths(locks: &[PoisonedLock]) -> [Constraint; 4] {

    [
        Constraint::Max(get_longest_len(locks.iter().map(|l| l.name()))),
        Constraint::Max(5),
        Constraint::Max(get_longest_len(
            locks
                .iter()
                .map(|l| l.get_poisoned_by().display_name.as_str()),
        ).max("Failed Task Name".len() as u16)),
        Constraint::Min(get_longest_len(
            locks
                .iter()
                .map(|l| l.get_poisoned_by().flake_ref.flake.as_str()),
        ).max("Task Flake URI".len() as u16)),
    ]
}

fn draw_task_picker(f: &mut Frame, locks: &[PoisonedLock], state: &mut TableState) {
    let rows: Vec<Row> = locks.iter().map(|l| l.into()).collect();

    // let widths = minimal_widths(locks);
    let widths = &[];
    let table = Table::new(rows, widths)
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
    let lock = lock.unwrap();
    draw_centered_popup(
        f,
        &format!("Do you really want to clear the lock {}?", lock.name()),
        button_select,
    );
}

fn ui(
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
