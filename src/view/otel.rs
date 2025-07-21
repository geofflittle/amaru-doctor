use crate::otel::span::AmaruSpan;
use chrono::TimeZone;
use chrono::Utc;
use color_eyre::Result;
use ratatui::layout::Constraint;
use ratatui::widgets::{Cell, Row, Table};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

pub fn _render_spans(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    spans: Vec<AmaruSpan>,
    is_focused: bool,
) -> Result<()> {
    let mut block = Block::default().title(title).borders(Borders::ALL);
    if is_focused {
        block = block
            .border_style(Style::default().fg(Color::Blue))
            .title_style(Style::default().fg(Color::White));
    }

    let rows = spans.iter().map(|event| {
        let timestamp = Utc
            .timestamp_nanos(
                event
                    .start
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as i64,
            )
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        Row::new(vec![
            Cell::from(event.trace_id.clone()),
            Cell::from(event.name.clone()),
            Cell::from(timestamp),
            Cell::from(format!(
                "{}µs",
                event
                    .end
                    .duration_since(event.start)
                    .unwrap_or_default()
                    .as_micros()
            )),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(32),
            Constraint::Length(20),
            Constraint::Length(26),
            Constraint::Length(10),
        ],
    )
    .header(Row::new(vec!["Trace ID", "Name", "Start Time", "Duration"]))
    .block(block)
    .column_spacing(1);

    frame.render_widget(table, area);

    Ok(())
}
