use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::tui::app::{App, AppMode};
use crate::der_parser::try_decode_input;

impl App {
    pub fn handle_input(&mut self, key: KeyEvent) {
        if self.show_help {
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
                    // Parse input buffer and update app state
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
                KeyCode::Char('d') => {},
                KeyCode::Char('a') => {},
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
}
