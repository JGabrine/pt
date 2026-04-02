use crate::app::{App, Focus, Status};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    draw_input(f, app, panes[0]);
    draw_output(f, app, panes[1]);
    draw_status(f, app, chunks[1]);
}

fn draw_input(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Input {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Input ")
        .border_style(border_style);

    app.input.set_block(block);
    f.render_widget(&app.input, area);
}

fn draw_output(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Output {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if let Some(ref breakdown) = app.why_breakdown {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let refined_block = Block::default()
            .borders(Borders::ALL)
            .title(" Refined ")
            .border_style(border_style);
        let refined = Paragraph::new(app.output.as_str())
            .block(refined_block)
            .wrap(Wrap { trim: false });
        f.render_widget(refined, parts[0]);

        let why_block = Block::default()
            .borders(Borders::ALL)
            .title(" Why ")
            .border_style(border_style);
        let why = Paragraph::new(breakdown.as_str())
            .block(why_block)
            .wrap(Wrap { trim: false });
        f.render_widget(why, parts[1]);
    } else {
        let output_block = Block::default()
            .borders(Borders::ALL)
            .title(" Output ")
            .border_style(border_style);
        let output = Paragraph::new(app.output.as_str())
            .block(output_block)
            .wrap(Wrap { trim: false })
            .scroll((app.scroll_offset, 0));
        f.render_widget(output, area);
    }
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let status_style = match app.status {
        Status::Idle => Style::default().fg(Color::Green),
        Status::Refining => Style::default().fg(Color::Yellow),
        Status::Error => Style::default().fg(Color::Red),
    };

    let key_style = Style::default().fg(Color::Cyan);

    let line = Line::from(vec![
        Span::styled(format!(" {} ", app.status_msg), status_style),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" ^R", key_style),
        Span::raw(" Refine  "),
        Span::styled("^W", key_style),
        Span::raw(" Why  "),
        Span::styled("^Y", key_style),
        Span::raw(" Copy  "),
        Span::styled("^L", key_style),
        Span::raw(" Clear  "),
        Span::styled("Tab", key_style),
        Span::raw(" Focus  "),
        Span::styled("^C", key_style),
        Span::raw(" Quit"),
    ]);

    let status_bar = Paragraph::new(line).style(Style::default().bg(Color::Rgb(30, 30, 30)));
    f.render_widget(status_bar, area);
}
