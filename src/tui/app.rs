use std::collections::HashSet;
use crate::der_parser::OwnedObject;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub parsed_objects: Vec<OwnedObject>,
    pub selected_path: Vec<usize>,
    pub collapsed_nodes: HashSet<Vec<usize>>,
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
            collapsed_nodes: HashSet::new(),
            show_help: false,
        }
    }
}
