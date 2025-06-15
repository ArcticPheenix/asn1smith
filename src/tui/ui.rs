use ratatui::{prelude::*, text::Span, widgets::*};
use crate::tui::app::App;
use crate::tui::tree::{tui_list_items};
use ratatui::widgets::Clear;
use ratatui::layout::Alignment;
use ratatui::widgets::BorderType;

impl App {
    pub fn draw(&self, f: &mut Frame) {
        let is_input_mode = matches!(self.mode, crate::tui::app::AppMode::Input);
        let constraints = if is_input_mode {
            [Constraint::Min(10), Constraint::Length(8)] // Large input, small tree
        } else {
            [Constraint::Length(3), Constraint::Min(10)] // Small input, large tree
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        self.draw_input(f, chunks[0]);
        self.draw_tree(f, chunks[1]);

        if self.show_help {
            self.draw_help_modal(f);
        }
        if self.should_show_hex_modal() {
            self.draw_hex_modal(f);
        }
    }

    fn should_show_hex_modal(&self) -> bool {
        matches!(self.mode, crate::tui::app::AppMode::View)
            && self.get_selected_object().is_some()
            && self.show_hex_modal
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
        let (items, selected_idx) = tui_list_items(&self.parsed_objects, &self.selected_path, &self.collapsed_nodes);
        let height = area.height as usize;
        let total_items = items.len();
        let mut scroll = self.tree_scroll;
        // Ensure scroll is always valid and the selected item is visible
        if total_items <= height {
            scroll = 0;
        } else if selected_idx < scroll {
            scroll = selected_idx;
        } else if selected_idx >= scroll + height {
            scroll = selected_idx + 1 - height;
        }
        if scroll + height > total_items {
            scroll = total_items.saturating_sub(height);
        }
        let end = (scroll + height).min(total_items);
        let visible_items = items[scroll..end].to_vec();
        let list = List::new(visible_items)
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(list, area);
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
            "  Tab       Switch to Input",
            "  j/k       Down/Up (navigate)",
            "  h/l       Collapse/Expand node",
            "  d         Delete node (not implemented)",
            "  a         Add child (not implemented)",
            "  x         Show hex modal for selected item",
            "  Esc       Close hex modal",
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

    pub fn draw_hex_modal(&self, f: &mut Frame) {
        use ratatui::widgets::{Block, Borders, Paragraph, Clear};
        let area = centered_rect(70, 60, f.area());
        let Some(obj) = self.get_selected_object() else { return; };
        let hex = get_object_hex_recursive(obj);
        let hex_str = hex.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        let paragraph = Paragraph::new(hex_str)
            .block(Block::default().borders(Borders::ALL).title("Hex View").border_type(BorderType::Double));
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }
}

fn get_object_hex_recursive(obj: &crate::der_parser::OwnedObject) -> Vec<u8> {
    match &obj.value {
        crate::der_parser::OwnedValue::Primitive(bytes) => bytes.clone(),
        crate::der_parser::OwnedValue::Constructed(children) => {
            let mut out = Vec::new();
            // Add this object's own bytes if available (if you want to include tag/length, you may need to store them)
            for child in children {
                out.extend(get_object_hex_recursive(child));
            }
            out
        }
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
