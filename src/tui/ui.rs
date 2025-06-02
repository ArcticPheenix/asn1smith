use ratatui::{prelude::*, text::Span, widgets::*};
use crate::tui::app::App;
use crate::tui::tree::{tui_list_items};
use ratatui::widgets::Clear;
use ratatui::layout::Alignment;
use ratatui::widgets::BorderType;

impl App {
    pub fn draw(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
            ])
            .split(f.area());

        self.draw_input(f, chunks[0]);
        self.draw_tree(f, chunks[1]);
        self.draw_hex(f, chunks[2]);

        if self.show_help {
            self.draw_help_modal(f);
        }
    }

    pub fn draw_input(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, crate::tui::app::AppMode::Input);
        let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("Input", active_style)
        } else {
            Span::raw("Input")
        };

        let paragraph = Paragraph::new(self.input_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
            )
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    pub fn draw_tree(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, crate::tui::app::AppMode::View);
        let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("ASN.1 Tree View", active_style)
        } else {
            Span::raw("ASN.1 Tree View")
        };
        let items = tui_list_items(&self.parsed_objects, &self.selected_path, &self.collapsed_nodes);
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(list, area);
    }

    pub fn draw_hex(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, crate::tui::app::AppMode::Hex);
        let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("Raw DER Hex View", active_style)
        } else {
            Span::raw("Raw DER Hex View")
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title);
        f.render_widget(block, area);
    }

    pub fn draw_help_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 60, f.area());
        let help_text = vec![
            "Help - Key Bindings",
            "",
            "General:",
            "  q         Quit",
            "  ?         Show this help",
            "",
            "Input Mode:",
            "  Ctrl-R    Parse input",
            "  Ctrl-U    Clear input",
            "  Tab/Esc   Switch to View",
            "  Enter     Newline",
            "  Any char  Add to buffer",
            "",
            "View Mode:",
            "  i         Switch to Input",
            "  Tab       Switch to Hex",
            "  h/l       Collapse/Expand node",
            "  j/k       Down/Up (navigate)",
            "  d         Delete node (not implemented)",
            "  a         Add child (not implemented)",
            "",
            "Hex Mode:",
            "  Tab       Switch to Input",
            "",
            "Press any key to close this help."
        ];
        let paragraph = Paragraph::new(help_text.join("\n")).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .border_type(BorderType::Double)
        ).alignment(Alignment::Left);
        f.render_widget(Clear, area); // Clear the area first
        f.render_widget(paragraph, area);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let vertical = popup_layout[1];
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical);
    horizontal_layout[1]
}
