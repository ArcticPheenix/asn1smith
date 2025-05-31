// src/tui_app.rs

use ratatui::{prelude::*, text::{Span}, widgets::*};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use base64::Engine;

use crate::der_parser::{ASN1Object, OwnedObject};
use crate::format::tui_list_items;

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
        }
    }

    fn move_selection_up(&mut self) {
        if let Some(current_idx) = self.selected_path.last_mut() {
            if *current_idx > 0 {
                *current_idx -= 1;
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
            if let Some(obj) = self.get_selected_object() {
                let parent_path = if self.selected_path.len() > 1 {
                    &self.selected_path[..self.selected_path.len() - 1]
                } else {
                    &[]
                };

                let mut current = if parent_path.is_empty() {
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
        match self.mode {
            AppMode::Input => match key.code {
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
                _ => {}
            },
            AppMode::Hex => match key.code {
                KeyCode::Tab => self.mode = AppMode::Input,
                KeyCode::Char('q') => self.should_quit = true,
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

fn draw_tree(&self, f: &mut Frame, area: Rect) {
    let is_active = matches!(self.mode, AppMode::View);
    let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let title = if is_active {
        Span::styled("ASN.1 Tree View", active_style)
    } else {
        Span::raw("ASN.1 Tree View")
    };
    let items = tui_list_items(&self.parsed_objects, &self.selected_path);
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

fn render_object<'a>(
    object: &OwnedObject,
    depth: usize,
    path: &mut Vec<usize>,
    selected_path: &[usize],
    items: &mut Vec<ListItem<'a>>,
) {
    use ratatui::style::{Style, Color, Modifier};

    let indent = "  ".repeat(depth);
    let label = format!("{}{}: {}", indent, object.tag.number, match &object.value {
        crate::der_parser::OwnedValue::Primitive(bytes) => format!("{:?}", bytes),
        crate::der_parser::OwnedValue::Constructed(_) => "Constructed".to_string(),
    });

    let is_selected = path == selected_path;
    let item = if is_selected {
        ListItem::new(label).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        ListItem::new(label)
    };
    items.push(item);

    if let crate::der_parser::OwnedValue::Constructed(children) = &object.value {
        for (i, child) in children.iter().enumerate() {
            path.push(i);
            render_object(child, depth + 1, path, selected_path, items);
            path.pop();
        }
    }
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
