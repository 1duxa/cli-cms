use dioxus::prelude::*;
use std::collections::HashMap;
use super::component::EDITOR_STATE;

// Buffer of unsaved style edits per component (ordered)
pub static STYLE_EDIT_BUFFER: GlobalSignal<HashMap<usize, Vec<(String, String)>>> = Signal::global(HashMap::new);

#[component]
pub fn StyleInput(component_id: usize) -> Element {
    let state = EDITOR_STATE.read();
    let component = state.components.get(&component_id);

    if component.is_none() {
        return rsx!(div { "Component not found" });
    }
    let component = component.unwrap();

    // Initialize buffer for this component if not present
    {
        let mut buf = STYLE_EDIT_BUFFER.write();
        if !buf.contains_key(&component_id) {
            buf.insert(component_id, component.styles.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>());
        }
    }

    // Read a snapshot for rendering
    let pairs_snapshot = { let buf = STYLE_EDIT_BUFFER.read(); buf.get(&component_id).cloned().unwrap_or_default() };

    rsx! {
        div { 
            class: "styles-editor",
            for (i, (key, value)) in pairs_snapshot.iter().enumerate() {
                div {
                    input {
                        value: "{key}",
                        oninput: move |e| {
                            let mut buf = STYLE_EDIT_BUFFER.write();
                            if let Some(vec) = buf.get_mut(&component_id) {
                                vec[i].0 = e.value();
                            }
                        }
                    }
                    input {
                        value: "{value}",
                        oninput: move |e| {
                            let mut buf = STYLE_EDIT_BUFFER.write();
                            if let Some(vec) = buf.get_mut(&component_id) {
                                vec[i].1 = e.value();
                            }
                        }
                    }
                    button {
                        onclick: move |_| {
                            let mut buf = STYLE_EDIT_BUFFER.write();
                            if let Some(vec) = buf.get_mut(&component_id) {
                                if i < vec.len() { vec.remove(i); }
                            }
                        },
                        "X"
                    }
                }
            }

            div { style: "margin-top: 8px; display:flex; gap:8px;",
                button {
                    onclick: move |_| {
                        let mut buf = STYLE_EDIT_BUFFER.write();
                        let vec = buf.entry(component_id).or_default();
                        let mut new_key = "new-property".to_string();
                        let mut counter = 1;
                        while vec.iter().any(|(k, _)| k == &new_key) {
                            new_key = format!("new-property-{}", counter);
                            counter += 1;
                        }
                        vec.push((new_key, "".to_string()));
                    },
                    "Add style"
                }

                button {
                    onclick: move |_| {
                        // Save: write ordered pairs into the component's HashMap (duplicates keep last)
                        let pairs = { let buf = STYLE_EDIT_BUFFER.read(); buf.get(&component_id).cloned().unwrap_or_default() };
                        let mut map = HashMap::new();
                        for (k, v) in pairs.iter() {
                            if !k.is_empty() {
                                map.insert(k.clone(), v.clone());
                            }
                        }
                        let mut s = EDITOR_STATE.write();
                        if let Some(comp) = s.components.get_mut(&component_id) {
                            comp.styles = map;
                        }
                        // remove buffer entry so next open loads fresh
                        STYLE_EDIT_BUFFER.write().remove(&component_id);
                    },
                    "Save"
                }

                button {
                    onclick: move |_| {
                        // Cancel: reset local edits from current component styles
                        let s = EDITOR_STATE.read();
                        if let Some(comp) = s.components.get(&component_id) {
                            let reset = comp.styles.iter().map(|(k,v)| (k.clone(), v.clone())).collect::<Vec<_>>();
                            STYLE_EDIT_BUFFER.write().insert(component_id, reset);
                        }
                    },
                    "Cancel"
                }
            }
        }
    }
}
fn update_style<A>(component_id: usize, property: A, value: String) where A: Into<String> {
    let property = property.into();
    let mut state = EDITOR_STATE.write();
    if let Some(component) = state.components.get_mut(&component_id) {
        if value.is_empty() {
            component.styles.remove(&property);
        } else {
            component.styles.insert(property, value);
        }
    }
}