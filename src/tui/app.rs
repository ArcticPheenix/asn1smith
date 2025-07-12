// src/tui/app.rs
use crate::der_parser::OwnedObject;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Input,
    View,
}

pub struct App {
    pub mode: AppMode,
    pub input_buffer: String,
    pub should_quit: bool,
    pub buffer: Vec<u8>,
    pub parsed_objects: Vec<OwnedObject>,
    pub selected_path: Vec<usize>,
    pub collapsed_nodes: HashSet<Vec<usize>>,
    pub show_help: bool,
    pub tree_scroll: usize,
    pub show_hex_modal: bool,
    pub copy_hex_to_clipboard: bool, // New field
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
            collapsed_nodes: HashSet::new(),
            show_help: false,
            tree_scroll: 0,
            show_hex_modal: false,
            copy_hex_to_clipboard: false, // Initialize
        }
    }
}
