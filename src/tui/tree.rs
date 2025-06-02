use crate::der_parser::{OwnedObject, TagClass};
use ratatui::widgets::ListItem;
use std::collections::HashSet;
use crate::tui::app::App;

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
) -> Vec<ListItem<'a>> {
    let mut items = Vec::new();
    let mut path = vec![0];
    for (i, obj) in objects.iter().enumerate() {
        path[0] = i;
        render_object(obj, 0, &mut path, selected_path, &mut items, collapsed_nodes);
    }
    items
}

fn render_object<'a>(
    object: &OwnedObject,
    depth: usize,
    path: &mut Vec<usize>,
    selected_path: &[usize],
    items: &mut Vec<ListItem<'a>>,
    collapsed_nodes: &HashSet<Vec<usize>>,
) {
    use ratatui::style::{Style, Color, Modifier};
    let indent = "  ".repeat(depth);
    let tag_display = if let Some(name) = tag_name(&object.tag.class, object.tag.number) {
        format!("{} ({})", name, object.tag.number)
    } else {
        object.tag.number.to_string()
    };
    let (label, is_collapsed) = match &object.value {
        crate::der_parser::OwnedValue::Primitive(bytes) => {
            let string_value = match (&object.tag.class, object.tag.number) {
                (TagClass::Universal, 19) | // PrintableString
                (TagClass::Universal, 20) | // T61String
                (TagClass::Universal, 22) | // IA5String
                (TagClass::Universal, 23) | // UTCTime
                (TagClass::Universal, 24)   // GeneralizedTime
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
                render_object(child, depth + 1, path, selected_path, items, collapsed_nodes);
                path.pop();
            }
        }
    }
}

impl App {
    pub fn move_selection_up(&mut self) {
        if let Some(current_idx) = self.selected_path.last_mut() {
            if *current_idx > 0 {
                *current_idx -= 1;
            } else if self.selected_path.len() > 1 {
                self.selected_path.pop();
            }
        }
    }

    pub fn move_selection_down(&mut self) {
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
            self.selected_path.pop();
            self.move_selection_down();
        }
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
}
