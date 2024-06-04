use std::io;

use crossterm::{event, execute};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppContext, humanize, LocksArgs, ResolveArgs};
use crate::error::Error;
use crate::http_client::prepare_client;

// TODO: REFACTOR EVERYTHING

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    New,
    Running,
    Finished(TaskResult),
}

type FlakeURI = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlakeRef {
    pub flake: FlakeURI,
    pub args: Vec<String>,
}

type Maow = u8; // this does not exist. look away. it's all for a reason.

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub locks: Vec<Maow>,
    pub flake_ref: FlakeRef,
    pub features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PoisonedLock {
    Write { id: String, name: String, poisoned: Task },
    Read { id: String, name: String, poisoned: Task },
}

impl PoisonedLock {

    pub fn id(&self) -> &str {
        match self {
            PoisonedLock::Write { id, .. } => id,
            PoisonedLock::Read { id, .. } => id,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            PoisonedLock::Write { name, .. } => name,
            PoisonedLock::Read { name, .. } => name,
        }
    }

    pub fn get_poisoned_by(&self) -> &Task {
        match self {
            PoisonedLock::Write { poisoned, .. } => poisoned,
            PoisonedLock::Read { poisoned, .. } => poisoned,
        }
    }

    pub fn get_type(&self) -> &'static str {
        match self {
            PoisonedLock::Write { .. } => "WRITE",
            PoisonedLock::Read { .. } => "READ",
        }
    }
}

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

enum LockType {
    Poisoned,
    Active,
}

impl From<&LocksArgs> for LockType {
    fn from(value: &LocksArgs) -> Self {
        match (value.poisoned, value.active) {
            (true, false) | (false, false) => LockType::Poisoned,
            (false, true) => LockType::Active,
            (_, _) => panic!("Cannot use active and poisoned flags at the same time."),
        }
    }
}

fn get_locks_endpoint(lock_type: LockType, detailed: bool) -> &'static str {
    match lock_type {
        LockType::Poisoned => match detailed {
            false => "api/v1/locks/poisoned",
            true => "api/v1/locks/poisoned_detailed",
        },
        LockType::Active => "api/v1/locks/active",
    }
}

fn fetch_locks_raw(ctx: &AppContext, lock_type: LockType, detailed: bool) -> Result<String, Error> {
    let client = prepare_client(ctx)?;
    let request = client
        .get(format!(
            "{}/{}",
            ctx.vicky_url,
            get_locks_endpoint(lock_type, detailed)
        ))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let locks = response.text()?;
    Ok(locks)
}

fn fetch_detailed_poisoned_locks(ctx: &AppContext) -> Result<Vec<PoisonedLock>, Error> {
    let raw_locks = fetch_locks_raw(ctx, LockType::Poisoned, true)?;
    let locks: Vec<PoisonedLock> = serde_json::from_str(&raw_locks)?;
    Ok(locks)
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

    let mut should_quit = false;
    while !should_quit {
        should_quit = handle_events(&mut state, locks.len(), &mut selected_task)?;
        if let Some(task_idx) = selected_task {
            unlock_lock(&resolve_args.ctx, &locks[task_idx])?;
            locks = fetch_detailed_poisoned_locks(&resolve_args.ctx)?;
            selected_task = None;
            // TODO: Add confirmation popup
        }
        terminal.draw(|f| ui(f, &locks, &mut state, &selected_task))?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

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

fn handle_events(
    state: &mut TableState,
    lock_amount: usize,
    selected_task: &mut Option<usize>,
) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    return Ok(true);
                }

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
        }
    }
    Ok(false)
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

fn draw_task_picker(
    f: &mut Frame,
    locks: &[PoisonedLock],
    state: &mut TableState,
) {
    let rows: Vec<Row> = locks.iter().map(|l| l.into()).collect();

    // let _widths: [Constraint; 4] = [
    //     Constraint::Max(get_longest_len(locks.iter().map(|l| l.name()))),
    //     Constraint::Max(5),
    //     Constraint::Max(get_longest_len(
    //         locks
    //             .iter()
    //             .map(|l| l.get_poisoned_by().display_name.as_str()),
    //     ).max("Failed Task Name".len() as u16)),
    //     Constraint::Min(get_longest_len(
    //         locks
    //             .iter()
    //             .map(|l| l.get_poisoned_by().flake_ref.flake.as_str()),
    //     ).max("Task Flake URI".len() as u16)),
    // ]; // This will not make the table equally spaced, but instead use minimal space. Just keeping this since I don't wanna rewrite it.
    let table = Table::new(rows, &[])
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

fn draw_confirm_clear(_f: &mut Frame, locks: &[PoisonedLock], selected: usize) {
    let lock = locks.get(selected);
    if lock.is_none() {
        return;
    }
    let _lock = lock.unwrap();
    todo!()
}

pub fn ui(
    f: &mut Frame,
    locks: &[PoisonedLock],
    state: &mut TableState,
    selected_task: &Option<usize>,
) {
    match selected_task {
        None => draw_task_picker(f, locks, state),
        Some(selected) => draw_confirm_clear(f, locks, *selected),
    }
}

