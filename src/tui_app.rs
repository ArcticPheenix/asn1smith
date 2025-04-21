// src/tui_app.rs

use ratatui::{prelude::*, widgets::*};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

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
        }
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Input => match key.code {
                KeyCode::Esc => self.mode = AppMode::View,
                KeyCode::Char(c) => self.input_buffer.push(c),
                KeyCode::Backspace => { self.input_buffer.pop(); }
                KeyCode::Tab => self.mode = AppMode::View,
                KeyCode::Enter => {
                    self.input_buffer.push('\n');
                },
                KeyCode::Char('p') => {
                    eprintln!("Parsing pasted input");
                    eprintln!("Raw input buffer: {}", self.input_buffer);
                    if let Ok(decoded) = try_decode_input(&self.input_buffer) {
                        self.buffer = decoded;
                        let mut parser = crate::der_parser::DerParser::new(&self.buffer);
                        match parser.parse_all() {
                            Ok(borrowed_objs) => {
                                self.parsed_objects = borrowed_objs.iter().map(crate::der_parser::OwnedObject::from).collect();
                                self.selected_path = vec![0];
                                self.mode = AppMode::View;
                            }
                            Err(e) => eprintln!("Parse failed: {:?}", e),
                        }
                    } else {
                        eprintln!("Input decoding failed.");
                    }
                },
                _ => {}
            },
            AppMode::View => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('i') => self.mode = AppMode::Input,
                KeyCode::Char('h') => {} // collapse
                KeyCode::Char('l') => {} // expand
                KeyCode::Char('j') => {} // move down
                KeyCode::Char('k') => {} // move up
                KeyCode::Char('d') => {} // delete
                KeyCode::Char('a') => {} // add child
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
        let paragraph = Paragraph::new(self.input_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn draw_tree(&self, f: &mut Frame, area: Rect) {
        let mut items = vec![];
        let mut path = vec![];
    
        for (i, obj) in self.parsed_objects.iter().enumerate() {
            path.push(i);
            render_object(obj, 0, &mut path, &self.selected_path, &mut items);
            path.pop();
        }
    
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("ASN.1 Tree View"));
    
        f.render_widget(list, area);
    }

    fn draw_hex(&self, f: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Raw DER Hex View");
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
    if let Ok(bytes) = base64::decode(&cleaned) {
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
    eprintln!("Rendering tag {} at depth {}", object.tag.number, depth);
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
