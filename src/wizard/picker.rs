// === The two screens every wizard question is built from

/* Every question in the wizard is one of exactly two shapes: choose one, or
 * choose any number. Two functions rather than one configurable widget,
 * because a single-select and a multi-select differ in what a key press
 * means (Enter commits one item versus commits the whole set), and a shared
 * function would need a mode flag threading that distinction through anyway. */

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

const HELP_SINGLE: &str = "up/down or j/k to move, enter to choose, esc to cancel the wizard";
const HELP_MULTI: &str = "up/down or j/k to move, space to toggle, enter to confirm, esc to cancel";

/// Choose exactly one option, starting on `default`. `Ok(None)` means the
/// user cancelled the whole wizard, not just this screen: there is no
/// "go back" in this milestone, only "start over" or "give up entirely",
/// since a partial answer set has nowhere else to live once the terminal
/// this drew it in is gone.
pub fn pick_single(
    terminal: &mut DefaultTerminal,
    title: &str,
    options: &[String],
    default: usize,
) -> Result<Option<usize>> {
    let mut state = ListState::default();
    state.select(Some(default.min(options.len().saturating_sub(1))));

    loop {
        terminal.draw(|frame| draw(frame, title, options, &mut state, &[], HELP_SINGLE))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up | KeyCode::Char('k') => move_selection(&mut state, options.len(), -1),
            KeyCode::Down | KeyCode::Char('j') => move_selection(&mut state, options.len(), 1),
            KeyCode::Enter => return Ok(state.selected()),
            _ => {}
        }
    }
}

/// Choose any number of options, none, or all. `checked` starts each option
/// pre-toggled per the layer's own `default` flag, the same defaults a
/// flag-driven run would have picked without the wizard.
pub fn pick_multi(
    terminal: &mut DefaultTerminal,
    title: &str,
    options: &[String],
    checked: &[bool],
) -> Result<Option<Vec<usize>>> {
    let mut state = ListState::default();
    state.select(Some(0));
    let mut checked = checked.to_vec();

    loop {
        terminal.draw(|frame| draw(frame, title, options, &mut state, &checked, HELP_MULTI))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up | KeyCode::Char('k') => move_selection(&mut state, options.len(), -1),
            KeyCode::Down | KeyCode::Char('j') => move_selection(&mut state, options.len(), 1),
            KeyCode::Char(' ') => {
                if let Some(i) = state.selected() {
                    checked[i] = !checked[i];
                }
            }
            KeyCode::Enter => {
                return Ok(Some(
                    checked
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &on)| on.then_some(i))
                        .collect(),
                ));
            }
            _ => {}
        }
    }
}

fn move_selection(state: &mut ListState, len: usize, delta: i32) {
    if len == 0 {
        return;
    }
    let current = state.selected().unwrap_or(0) as i32;
    let next = (current + delta).rem_euclid(len as i32);
    state.select(Some(next as usize));
}

fn draw(
    frame: &mut ratatui::Frame,
    title: &str,
    options: &[String],
    state: &mut ListState,
    checked: &[bool],
    help: &str,
) {
    let area = frame.area();
    let [list_area, help_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let mark = match checked.get(i) {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            };
            ListItem::new(Line::from(format!("{mark}{label}")))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} ")),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, list_area, state);
    frame.render_widget(Paragraph::new(help), help_area);
}
