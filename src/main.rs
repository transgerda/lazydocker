mod app;
mod container;
mod docker;
mod widget;

use app::App;
use arboard::Clipboard;
use crossterm::event::{self, Event, EventStream, KeyCode};
use futures_util::StreamExt;
use notify_rust::Notification;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType::Rounded, Borders, List, ListItem, ListState, Paragraph},
};
use std::time::Duration;
use widget::Widgets;

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = app(&mut terminal).await;
    ratatui::restore();
    result?;
    Ok(())
}

async fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new().await;

    let refresh_interval = Duration::from_millis(250);
    let mut ticker = tokio::time::interval(refresh_interval);
    let mut events = EventStream::new();

    loop {
        terminal.draw(|frame| render(frame, &mut app))?;

        tokio::select! {
            _ = ticker.tick() => {
                app.refresh().await?;
            }

            maybe_event = events.next() => {
                let Some(Ok(Event::Key(key))) = maybe_event else {
                    continue;
                };

                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => break,

                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => break,
                    KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => break,

                    KeyCode::Char('1') => app.active_widget = Widgets::Containers,
                    KeyCode::Char('2') => app.active_widget = Widgets::TTY,
                    KeyCode::Char('3') => app.active_widget = Widgets::Logs,

                    KeyCode::Char('h') => app.active_widget = app.active_widget.prev(),
                    KeyCode::Char('l') => app.active_widget = app.active_widget.next(),
                    KeyCode::Left => app.active_widget = app.active_widget.prev(),
                    KeyCode::Right => app.active_widget = app.active_widget.next(),

                    KeyCode::Char('S') => {
                        docker::stop(&app.current_container().id).await;
                    }

                    KeyCode::Char('s') => {
                        docker::start(&app.current_container().id).await;
                    }

                    KeyCode::Char('R') => {
                        docker::restart(&app.current_container().id).await;
                    }

                    // Manually refresh
                    KeyCode::Char('r') => {
                        app.get_containers().await;

                        // Make sure selection is still valid
                        if app.containers.is_empty() {
                            app.active_container = 0;
                        } else if app.active_container >= app.containers.len() {
                            app.active_container = app.containers.len() - 1;
                        }

                        app.scroll_offset = 0;
                    }

                    KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => {
                        match app.active_widget {
                            Widgets::Containers => {
                                if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                    app.next_container().await;
                                } else {
                                    app.scroll_down_logs(10);
                                }
                            }

                            Widgets::TTY => {}

                            Widgets::Logs => {
                                if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                    app.scroll_down_logs(1);
                                } else {
                                    app.scroll_down_logs(10);
                                }
                            }
                        }
                    }

                    KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match app.active_widget {
                        Widgets::Containers => {
                            if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                app.previous_container().await;
                            } else {
                                app.scroll_up_logs(10);
                            }
                        }

                        Widgets::TTY => {}

                        Widgets::Logs => {
                            if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                app.scroll_up_logs(1);
                            } else {
                                app.scroll_up_logs(10);
                            }
                        }
                    },

                    KeyCode::Char('g') | KeyCode::Char('G') => match app.active_widget {
                        Widgets::Containers => {
                            if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                app.set_container(0).await;
                            } else {
                                app.set_container(app.containers.len().saturating_sub(1)).await;
                            }
                        }

                        Widgets::TTY => {}

                        Widgets::Logs => {
                            if !key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                app.scroll_offset = 0;
                            } else {
                                app.refresh_logs().await;
                                let max_offset = app
                                    .current_container()
                                    .logs
                                    .lines()
                                    .count()
                                    .saturating_sub(1);
                                app.scroll_offset = max_offset;
                            }
                        }
                    },

                    KeyCode::Char('d') => match app.active_widget {
                        Widgets::Containers => {}

                        Widgets::TTY => {}

                        Widgets::Logs => {
                            app.scroll_up_logs(10);
                        }
                    },

                    KeyCode::Char('u') => match app.active_widget {
                        Widgets::Containers => {}

                        Widgets::TTY => {}

                        Widgets::Logs => {
                            app.scroll_up_logs(10);
                        }
                    },

                    KeyCode::Char('y') => match app.active_widget {
                        Widgets::Containers => {
                            let mut clipboard =
                                Clipboard::new().map_err(|e| std::io::Error::other(e.to_string()))?;

                            let id = &app.containers[app.active_container].id;
                            clipboard
                                .set_text(id)
                                .map_err(|e| std::io::Error::other(e.to_string()))?;

                            Notification::new()
                                .summary("Copied to clipboard")
                                .body(&format!(
                                    "Container id: {}",
                                    app.containers[app.active_container].id
                                ))
                                .show()
                                .unwrap();
                        }

                        Widgets::TTY => {}

                        Widgets::Logs => {}
                    },

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    // Split the main area (outer[0]) into left 1/3 and right 2/3
    let main_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(outer[0]);

    // Split the left third into top/bottom halves
    let left_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(main_areas[0]);

    let footer_area = outer[1];
    let logs_area = main_areas[1];
    let containers_area = left_areas[0];
    let tty_area = left_areas[1];

    let containers_widget = List::new(
        app.containers
            .iter()
            .map(|container| {
                ListItem::new(Text::from(vec![
                    Line::from(vec![Span::styled(
                        container.name.clone(),
                        Style::default()
                            .fg(if container.status.starts_with("Up") {
                                Color::Green
                            } else {
                                Color::Cyan
                            })
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(vec![
                        Span::styled("  Status: ", Style::default().fg(Color::Blue)),
                        Span::raw(container.status.as_str()),
                    ]),
                    Line::from(vec![
                        Span::styled("  Image:  ", Style::default().fg(Color::Blue)),
                        Span::raw(container.image.as_str()),
                    ]),
                ]))
            })
            .collect::<Vec<ListItem>>(),
    )
    .block(
        Block::default()
            .title(Span::styled(
                " [1] Containers ",
                Style::default().fg(Color::Gray),
            ))
            .borders(Borders::ALL)
            .border_type(Rounded)
            .border_style(
                Style::default().fg(if app.active_widget == Widgets::Containers {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }),
            ),
    )
    .highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol(" ► ");

    let mut container_state = ListState::default();
    container_state.select(Some(app.active_container));

    let tty_widget = Paragraph::new("").block(
        Block::default()
            .title(Span::styled(" [2] TTY ", Style::default().fg(Color::Gray)))
            .borders(Borders::ALL)
            .border_type(Rounded)
            .border_style(Style::default().fg(if app.active_widget == Widgets::TTY {
                Color::Cyan
            } else {
                Color::DarkGray
            })),
    );

    let visible = logs_area.height.saturating_sub(2); // minus top/bottom border
    let total = app.current_container().logs.lines().count() as u16;
    let max_scroll = total.saturating_sub(visible);

    let from_bottom = app.scroll_offset.min(max_scroll as usize) as u16;
    app.scroll_offset = from_bottom as usize;
    let scroll = max_scroll - from_bottom;

    let logs_widget = Paragraph::new(app.current_container().logs.clone())
        .block(
            Block::default()
                .title(Span::styled(" [3] Logs ", Style::default().fg(Color::Gray)))
                .borders(Borders::ALL)
                .border_type(Rounded)
                .border_style(Style::default().fg(if app.active_widget == Widgets::Logs {
                    Color::Cyan
                } else {
                    Color::DarkGray
                })),
        )
        .scroll((scroll as u16, 0));

    let footer = Paragraph::new(" h/l or ←/→: Swap widgets   r: Refresh   q: Quit   R: Restart   S: Stop   s: Start   y: Copy container id to clipboard ")
        .style(Style::default().fg(Color::Gray));

    frame.render_stateful_widget(containers_widget, containers_area, &mut container_state);
    frame.render_widget(footer, footer_area);
    frame.render_widget(tty_widget, tty_area);
    frame.render_widget(logs_widget, logs_area);
}
