// src/tui_app.rs

use ratatui::{prelude::*, text::{Span}, widgets::*};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use base64::Engine;

use crate::der_parser::{ASN1Object, OwnedObject};

pub enum AppMode {
    Input,
    View,
    Hex,
}

pub struct App {
    pub mode: AppMode,
    pub input_buffer: String,
    pub should_quit: bool,
    pub buffer: Vec<u8>,
    pub parsed_objects: Vec<crate::der_parser::OwnedObject>,
    pub selected_path: Vec<usize>,
    pub collapsed_nodes: std::collections::HashSet<Vec<usize>>,
    pub show_help: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Input,
            input_buffer: String::new(),
            should_quit: false,
            parsed_objects: Vec::new(),
            selected_path: vec![],
            buffer: Vec::new(),
            collapsed_nodes: std::collections::HashSet::new(),
            show_help: false,
        }
    }

    fn move_selection_up(&mut self) {
        // If we're at the first child in a constructed node, move up to the parent
        if let Some(current_idx) = self.selected_path.last_mut() {
            if *current_idx > 0 {
                *current_idx -= 1;
            } else if self.selected_path.len() > 1 {
                self.selected_path.pop();
            }
        }
    }

    fn move_selection_down(&mut self) {
        // First check if we can descend into children
        let can_descend = {
            let obj = self.get_selected_object();
            obj.map_or(false, |o| {
                if let crate::der_parser::OwnedValue::Constructed(ref children) = o.value {
                    !children.is_empty() && !self.collapsed_nodes.contains(&self.selected_path)
                } else {
                    false
                }
            })
        };

        if can_descend {
            self.selected_path.push(0);
            return;
        }

        // If we can't descend, try moving to the next sibling
        let next_sibling_valid = {
            if let Some(_obj) = self.get_selected_object() {
                let parent_path = if self.selected_path.len() > 1 {
                    &self.selected_path[..self.selected_path.len() - 1]
                } else {
                    &[]
                };

                let current = if parent_path.is_empty() {
                    &self.parsed_objects
                } else {
                    let mut parent = &self.parsed_objects[0];
                    for &idx in parent_path.iter().skip(1) {
                        if let crate::der_parser::OwnedValue::Constructed(ref children) = parent.value {
                            parent = &children[idx];
                        }
                    }
                    if let crate::der_parser::OwnedValue::Constructed(ref children) = parent.value {
                        children
                    } else {
                        return;
                    }
                };

                if let Some(idx) = self.selected_path.last() {
                    *idx + 1 < current.len()
                } else {
                    false
                }
            } else {
                false
            }
        };

        if next_sibling_valid {
            *self.selected_path.last_mut().unwrap() += 1;
        } else if self.selected_path.len() > 1 {
            // If no next sibling, move up to parent and try to move to its next sibling
            self.selected_path.pop();
            self.move_selection_down();
        }
    }

    fn toggle_collapse(&mut self) {
        if self.get_selected_object().map_or(false, |obj| {
            matches!(obj.value, crate::der_parser::OwnedValue::Constructed(_))
        }) {
            if !self.collapsed_nodes.remove(&self.selected_path) {
                self.collapsed_nodes.insert(self.selected_path.clone());
            }
        }
    }

    fn get_selected_object(&self) -> Option<&OwnedObject> {
        let mut current = self.parsed_objects.get(0)?;
        for &idx in self.selected_path.iter().skip(1) {
            if let crate::der_parser::OwnedValue::Constructed(children) = &current.value {
                current = children.get(idx)?;
            } else {
                return None;
            }
        }
        Some(current)
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        if self.show_help {
            // Any key closes the help modal
            self.show_help = false;
            return;
        }
        match self.mode {
            AppMode::Input => match key.code {
                KeyCode::Char('?') => self.show_help = true,
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input_buffer.clear();
                }
                KeyCode::Esc => self.mode = AppMode::View,
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    eprintln!("Ctrl-R pressed: parsing input");
                    eprintln!("Raw input buffer: {}", self.input_buffer);
                    if let Ok(decoded) = try_decode_input(&self.input_buffer) {
                        self.buffer = decoded;
                        let mut parser = crate::der_parser::DerParser::new(&self.buffer);
                        match parser.parse_all() {
                            Ok(borrowed_objs) => {
                                self.parsed_objects = borrowed_objs
                                    .iter()
                                    .map(crate::der_parser::OwnedObject::from)
                                    .collect();
                                self.selected_path = vec![0];
                                self.mode = AppMode::View;
                            }
                            Err(e) => eprintln!("Parse failed: {:?}", e),
                        }
                    } else {
                        eprintln!("Input decoding failed.");
                    }
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Tab => self.mode = AppMode::View,
                KeyCode::Enter => {
                    self.input_buffer.push('\n');
                }
                KeyCode::Char(c) => self.input_buffer.push(c),
                _ => {}
            },
            AppMode::View => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('i') => self.mode = AppMode::Input,
                KeyCode::Char('h') => self.toggle_collapse(),
                KeyCode::Char('l') => self.toggle_collapse(),
                KeyCode::Char('j') => self.move_selection_down(),
                KeyCode::Char('k') => self.move_selection_up(),
                KeyCode::Char('d') => {}, // delete
                KeyCode::Char('a') => {}, // add child
                KeyCode::Tab => self.mode = AppMode::Hex,
                KeyCode::Char('?') => self.show_help = true,
                _ => {}
            },
            AppMode::Hex => match key.code {
                KeyCode::Tab => self.mode = AppMode::Input,
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('?') => self.show_help = true,
                _ => {}
            },
        }
    }

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

    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, AppMode::Input);
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

    fn tag_name(class: &crate::der_parser::TagClass, number: u32) -> Option<&'static str> {
        match (class, number) {
            (crate::der_parser::TagClass::Universal, 1) => Some("BOOLEAN"),
            (crate::der_parser::TagClass::Universal, 2) => Some("INTEGER"),
            (crate::der_parser::TagClass::Universal, 3) => Some("BIT STRING"),
            (crate::der_parser::TagClass::Universal, 4) => Some("OCTET STRING"),
            (crate::der_parser::TagClass::Universal, 5) => Some("NULL"),
            (crate::der_parser::TagClass::Universal, 6) => Some("OBJECT IDENTIFIER"),
            (crate::der_parser::TagClass::Universal, 10) => Some("ENUMERATED"),
            (crate::der_parser::TagClass::Universal, 16) => Some("SEQUENCE"),
            (crate::der_parser::TagClass::Universal, 17) => Some("SET"),
            (crate::der_parser::TagClass::Universal, 19) => Some("PrintableString"),
            (crate::der_parser::TagClass::Universal, 20) => Some("T61String"),
            (crate::der_parser::TagClass::Universal, 22) => Some("IA5String"),
            (crate::der_parser::TagClass::Universal, 23) => Some("UTCTime"),
            (crate::der_parser::TagClass::Universal, 24) => Some("GeneralizedTime"),
            _ => None,
        }
    }

    fn render_object<'a>(
        object: &OwnedObject,
        depth: usize,
        path: &mut Vec<usize>,
        selected_path: &[usize],
        items: &mut Vec<ListItem<'a>>,
        collapsed_nodes: &std::collections::HashSet<Vec<usize>>,
    ) {
        use ratatui::style::{Style, Color, Modifier};

        let indent = "  ".repeat(depth);
        let tag_display = if let Some(name) = Self::tag_name(&object.tag.class, object.tag.number) {
            format!("{} ({})", name, object.tag.number)
        } else {
            object.tag.number.to_string()
        };
        let (label, is_collapsed) = match &object.value {
            crate::der_parser::OwnedValue::Primitive(bytes) => {
                // Show string value for string-based tags
                let string_value = match (&object.tag.class, object.tag.number) {
                    (crate::der_parser::TagClass::Universal, 19) | // PrintableString
                    (crate::der_parser::TagClass::Universal, 20) | // T61String
                    (crate::der_parser::TagClass::Universal, 22) | // IA5String
                    (crate::der_parser::TagClass::Universal, 23) | // UTCTime
                    (crate::der_parser::TagClass::Universal, 24)   // GeneralizedTime
                        => std::str::from_utf8(bytes).ok(),
                    _ => None,
                };
                let value_display = if let Some(s) = string_value {
                    format!("'{}'", s)
                } else {
                    format!("{:?}", bytes)
                };
                (format!("{}{}: {}", indent, tag_display, value_display), false)
            },
            crate::der_parser::OwnedValue::Constructed(children) => {
                let collapsed = collapsed_nodes.contains(path);
                let marker = if collapsed { "▶" } else { "▼" };
                (
                    format!("{}{} {}: Constructed ({} children)", indent, marker, tag_display, children.len()),
                    collapsed
                )
            }
        };

        let is_selected = path == selected_path;
        let item = if is_selected {
            ListItem::new(label).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            ListItem::new(label)
        };
        items.push(item);

        if let crate::der_parser::OwnedValue::Constructed(children) = &object.value {
            if !is_collapsed {
                for (i, child) in children.iter().enumerate() {
                    path.push(i);
                    Self::render_object(child, depth + 1, path, selected_path, items, collapsed_nodes);
                    path.pop();
                }
            }
        }
    }

    fn tui_list_items<'a>(
        objects: &'a [OwnedObject],
        selected_path: &[usize],
        collapsed_nodes: &std::collections::HashSet<Vec<usize>>,
    ) -> Vec<ListItem<'a>> {
        let mut items = Vec::new();
        let mut path = vec![0];
        for (i, obj) in objects.iter().enumerate() {
            path[0] = i;
            Self::render_object(obj, 0, &mut path, selected_path, &mut items, collapsed_nodes);
        }
        items
    }

    fn draw_tree(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, AppMode::View);
        let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let title = if is_active {
            Span::styled("ASN.1 Tree View", active_style)
        } else {
            Span::raw("ASN.1 Tree View")
        };
        let items = Self::tui_list_items(&self.parsed_objects, &self.selected_path, &self.collapsed_nodes);
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(list, area);
    }

    fn draw_hex(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.mode, AppMode::Hex);
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

    fn draw_help_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 60, f.size());
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

use ratatui::widgets::Clear;
use ratatui::layout::Alignment;
use ratatui::widgets::BorderType;

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn try_decode_input(input: &str) -> Result<Vec<u8>, ()> {
    let cleaned: String = input
        .lines()
        .filter(|line| !line.starts_with("-----")) // Strip PEM boundaries
        .collect::<Vec<_>>()
        .join("");

    // Try hex first
    if let Ok(bytes) = hex::decode(&cleaned) {
        return Ok(bytes);
    }

    // Try base64 next
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&cleaned) {
        return Ok(bytes);
    }

    Err(())
}

pub fn run_ui<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                app.handle_input(key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
