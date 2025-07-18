// src/tui/tree.rs
use crate::der_parser::{OwnedObject, TagClass};
use crate::tui::app::App;
use ratatui::widgets::ListItem;
use std::collections::HashSet;

pub fn tag_name(class: &TagClass, number: u32) -> Option<&'static str> {
    match (class, number) {
        (TagClass::Universal, 1) => Some("BOOLEAN"),
        (TagClass::Universal, 2) => Some("INTEGER"),
        (TagClass::Universal, 3) => Some("BIT STRING"),
        (TagClass::Universal, 4) => Some("OCTET STRING"),
        (TagClass::Universal, 5) => Some("NULL"),
        (TagClass::Universal, 6) => Some("OBJECT IDENTIFIER"),
        (TagClass::Universal, 10) => Some("ENUMERATED"),
        (TagClass::Universal, 16) => Some("SEQUENCE"),
        (TagClass::Universal, 17) => Some("SET"),
        (TagClass::Universal, 19) => Some("PrintableString"),
        (TagClass::Universal, 20) => Some("T61String"),
        (TagClass::Universal, 22) => Some("IA5String"),
        (TagClass::Universal, 23) => Some("UTCTime"),
        (TagClass::Universal, 24) => Some("GeneralizedTime"),
        _ => None,
    }
}

pub fn tui_list_items<'a>(
    objects: &'a [OwnedObject],
    selected_path: &[usize],
    collapsed_nodes: &HashSet<Vec<usize>>,
) -> (Vec<ListItem<'a>>, usize) {
    let mut items = Vec::new();
    let mut path = vec![0];
    let mut selected_idx = 0;
    for (i, obj) in objects.iter().enumerate() {
        path[0] = i;
        render_object_with_index(
            obj,
            0,
            &mut path,
            selected_path,
            &mut items,
            collapsed_nodes,
            &mut selected_idx,
        );
    }
    (items, selected_idx)
}

fn render_object_with_index<'a>(
    object: &OwnedObject,
    depth: usize,
    path: &mut Vec<usize>,
    selected_path: &[usize],
    items: &mut Vec<ListItem<'a>>,
    collapsed_nodes: &HashSet<Vec<usize>>,
    selected_idx: &mut usize,
) {
    use ratatui::style::{Color, Modifier, Style};
    let indent = "  ".repeat(depth);
    let tag_display = if let Some(name) = tag_name(&object.tag.class, object.tag.number) {
        format!("{} ({})", name, object.tag.number)
    } else {
        object.tag.number.to_string()
    };
    let (label, is_collapsed) = match &object.value {
        crate::der_parser::OwnedValue::Primitive(bytes) => {
            let string_value = match (&object.tag.class, object.tag.number) {
                (TagClass::Universal, 19) |
                (TagClass::Universal, 20) |
                (TagClass::Universal, 22) |
                (TagClass::Universal, 23) |
                (TagClass::Universal, 24)   // GeneralizedTime
                    => std::str::from_utf8(bytes).ok(),
                _ => None,
            };
            let value_display = if let Some(s) = string_value {
                format!("'{}'", s)
            } else {
                format!("{:?}", bytes)
            };
            (
                format!("{}{}: {}", indent, tag_display, value_display),
                false,
            )
        }
        crate::der_parser::OwnedValue::Constructed(children) => {
            let collapsed = collapsed_nodes.contains(path);
            let marker = if collapsed { "▶" } else { "▼" };
            (
                format!(
                    "{}{} {}: Constructed ({} children)",
                    indent,
                    marker,
                    tag_display,
                    children.len()
                ),
                collapsed,
            )
        }
    };
    let is_selected = path == selected_path;
    if is_selected {
        *selected_idx = items.len();
    }
    let item = if is_selected {
        ListItem::new(label).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ListItem::new(label)
    };
    items.push(item);
    if let crate::der_parser::OwnedValue::Constructed(children) = &object.value {
        if !is_collapsed {
            for (i, child) in children.iter().enumerate() {
                path.push(i);
                render_object_with_index(
                    child,
                    depth + 1,
                    path,
                    selected_path,
                    items,
                    collapsed_nodes,
                    selected_idx,
                );
                path.pop();
            }
        }
    }
}

impl App {
    pub fn move_selection_up(&mut self, area_height: usize) {
        if self.selected_path.is_empty() {
            return;
        }
        if let Some(current_idx) = self.selected_path.last_mut() {
            if *current_idx > 0 {
                *current_idx -= 1;
                // Move to the last visible descendant of the previous sibling
                while let Some(obj) = self.get_selected_object() {
                    if let crate::der_parser::OwnedValue::Constructed(children) = &obj.value {
                        if !children.is_empty()
                            && !self.collapsed_nodes.contains(&self.selected_path)
                        {
                            self.selected_path.push(children.len() - 1);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            } else if self.selected_path.len() > 1 {
                self.selected_path.pop();
            }
        }
        self.update_tree_scroll(area_height);
    }

    pub fn move_selection_down(&mut self, area_height: usize) {
        // Try to descend into children if possible
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
            self.update_tree_scroll(area_height);
            return;
        }
        // Otherwise, try to move to the next sibling or ancestor's next sibling
        let mut path = self.selected_path.clone();
        while !path.is_empty() {
            let check_path = {
                let mut p = path.clone();
                if let Some(last) = p.last_mut() {
                    *last += 1;
                }
                p
            };
            if get_object_by_path(&self.parsed_objects, &check_path).is_some() {
                self.selected_path = check_path;
                self.update_tree_scroll(area_height);
                return;
            }
            path.pop();
        }
        self.update_tree_scroll(area_height);
    }

    pub fn toggle_collapse(&mut self) {
        if self.get_selected_object().map_or(false, |obj| {
            matches!(obj.value, crate::der_parser::OwnedValue::Constructed(_))
        }) {
            if !self.collapsed_nodes.remove(&self.selected_path) {
                self.collapsed_nodes.insert(self.selected_path.clone());
            }
        }
    }

    pub fn get_selected_object(&self) -> Option<&OwnedObject> {
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

    /// Call this after changing selection to ensure selected item is visible.
    pub fn update_tree_scroll(&mut self, area_height: usize) {
        let (items, selected_idx) = crate::tui::tree::tui_list_items(
            &self.parsed_objects,
            &self.selected_path,
            &self.collapsed_nodes,
        );
        if selected_idx < self.tree_scroll {
            self.tree_scroll = selected_idx;
        } else if selected_idx >= self.tree_scroll + area_height {
            self.tree_scroll = selected_idx + 1 - area_height;
        }
    }
}

fn get_object_by_path<'a>(objects: &'a [OwnedObject], path: &[usize]) -> Option<&'a OwnedObject> {
    let mut current = objects.get(*path.get(0)?);
    for &idx in path.iter().skip(1) {
        current = match current {
            Some(obj) => match &obj.value {
                crate::der_parser::OwnedValue::Constructed(children) => children.get(idx),
                _ => return None,
            },
            None => return None,
        };
    }
    current
}
