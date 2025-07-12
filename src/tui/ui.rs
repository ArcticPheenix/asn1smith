// src/tui/ui.rs
use crate::tui::app::App;
use crate::tui::tree::tui_list_items;
use clipboard::{ClipboardContext, ClipboardProvider};
use ratatui::layout::Alignment;
use ratatui::widgets::BorderType;
use ratatui::widgets::Clear;
use ratatui::{
    prelude::*,
    style::Color,
    text::{Line, Span},
    widgets::*,
};

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
        } else if self.should_show_hex_modal() {
            self.draw_hex_modal(f);
        } else {
            self.draw_help_hint(f);
        }
    }

    fn should_show_hex_modal(&self) -> bool {
        matches!(self.mode, crate::tui::app::AppMode::View)
            && self.get_selected_object().is_some()
            && self.show_hex_modal
    }

    pub fn draw_input(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, crate::tui::app::AppMode::Input);
        let active_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("Input", active_style)
        } else {
            Span::raw("Input")
        };

        let paragraph = Paragraph::new(self.input_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    pub fn draw_tree(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, crate::tui::app::AppMode::View);
        let active_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("ASN.1 Tree View", active_style)
        } else {
            Span::raw("ASN.1 Tree View")
        };
        let (items, selected_idx) = tui_list_items(
            &self.parsed_objects,
            &self.selected_path,
            &self.collapsed_nodes,
        );
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
        let list =
            List::new(visible_items).block(Block::default().borders(Borders::ALL).title(title));
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
            "Hex Modal:",
            "  Ctrl-C    Copy hex to clipboard",
            "  Esc       Close hex modal",
            "",
            "Press any key to close this help.",
        ];
        let paragraph = Paragraph::new(help_text.join("\n"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        "Help",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_type(BorderType::Double),
            )
            .alignment(Alignment::Left);
        f.render_widget(Clear, area); // Clear the area first
        f.render_widget(paragraph, area);
    }

    pub fn draw_hex_modal(&self, f: &mut Frame) {
        use ratatui::widgets::{Block, Borders, Clear, Paragraph};
        let area = centered_rect(70, 60, f.area());
        let Some(obj) = self.get_selected_object() else {
            return;
        };
        let (tag_bytes, length_bytes, value_bytes) = get_tag_length_value_bytes(obj);
        let mut copied = false;
        // Compose colored spans
        let mut spans = vec![];
        if !tag_bytes.is_empty() {
            let tag_hex = tag_bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            spans.push(Span::styled(tag_hex, Style::default().fg(Color::Cyan)));
        }
        if !length_bytes.is_empty() {
            if !spans.is_empty() {
                spans.push(Span::raw(" "));
            }
            let len_hex = length_bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            spans.push(Span::styled(len_hex, Style::default().fg(Color::White)));
        }
        if !value_bytes.is_empty() {
            if !spans.is_empty() {
                spans.push(Span::raw(" "));
            }
            let val_hex = value_bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            spans.push(Span::styled(val_hex, Style::default().fg(Color::Green)));
        }
        if self.copy_hex_to_clipboard {
            let all_bytes = tag_bytes
                .iter()
                .chain(length_bytes.iter())
                .chain(value_bytes.iter())
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            if let Ok(mut ctx) = ClipboardContext::new() {
                let _ = ctx.set_contents(all_bytes);
                copied = true;
            }
        }
        let mut lines = vec![Line::from(spans)];
        if copied {
            lines.push(Line::from(vec![Span::styled(
                "Copied to clipboard!",
                Style::default().fg(Color::Yellow),
            )]));
        }
        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Hex View")
                .border_type(BorderType::Double),
        );
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    /// Draws a small help hint in the bottom right corner.
    fn draw_help_hint(&self, f: &mut Frame) {
        let area = f.area();
        let hint = "Press '?' for help";
        let width = hint.len() as u16 + 4;
        let height = 3u16;
        let x = area.x + area.width.saturating_sub(width);
        let y = area.y + area.height.saturating_sub(height);
        let rect = Rect {
            x,
            y,
            width,
            height,
        };
        let paragraph = Paragraph::new(hint)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, rect);
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

/// Extracts the tag, length, and value bytes for a single ASN.1 object.
fn get_tag_length_value_bytes(obj: &crate::der_parser::OwnedObject) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // This assumes the object was parsed from DER and the tag/length/value are contiguous in the original encoding.
    // If you have the original DER bytes, you should store them per object for perfect accuracy.
    // Here, we reconstruct them as best as possible from the object fields.
    use crate::der_parser::OwnedValue;
    let mut tag_bytes = vec![];
    let mut length_bytes = vec![];
    let mut value_bytes = vec![];
    // Tag encoding (single byte for most tags)
    let tag = &obj.tag;
    let mut first_byte = ((match tag.class {
        crate::der_parser::TagClass::Universal => 0b00,
        crate::der_parser::TagClass::Application => 0b01,
        crate::der_parser::TagClass::ContextSpecific => 0b10,
        crate::der_parser::TagClass::Private => 0b11,
    }) << 6) as u8;
    if tag.constructed {
        first_byte |= 0b0010_0000;
    }
    if tag.number < 31 {
        first_byte |= tag.number as u8;
        tag_bytes.push(first_byte);
    } else {
        first_byte |= 0b0001_1111;
        tag_bytes.push(first_byte);
        let mut n = tag.number;
        let mut stack = vec![];
        while n > 0 {
            stack.push((n & 0x7F) as u8);
            n >>= 7;
        }
        for (i, b) in stack.iter().rev().enumerate() {
            let mut byte = *b;
            if i != stack.len() - 1 {
                byte |= 0x80;
            }
            tag_bytes.push(byte);
        }
    }
    // Length encoding
    if obj.length < 128 {
        length_bytes.push(obj.length as u8);
    } else {
        let mut len = obj.length;
        let mut len_bytes = vec![];
        while len > 0 {
            len_bytes.push((len & 0xFF) as u8);
            len >>= 8;
        }
        len_bytes.reverse();
        length_bytes.push(0x80 | (len_bytes.len() as u8));
        length_bytes.extend(len_bytes);
    }
    // Value bytes
    match &obj.value {
        OwnedValue::Primitive(bytes) => value_bytes.extend(bytes),
        OwnedValue::Constructed(children) => {
            for child in children {
                let (t, l, v) = get_tag_length_value_bytes(child);
                value_bytes.extend(t);
                value_bytes.extend(l);
                value_bytes.extend(v);
            }
        }
    }
    (tag_bytes, length_bytes, value_bytes)
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
