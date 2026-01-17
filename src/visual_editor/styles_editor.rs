use dioxus::prelude::*;
use std::collections::HashMap;
use super::component::EDITOR_STATE;
#[component]
pub fn StyleInput(component_id: usize) -> Element {
    let state = EDITOR_STATE.read();
    let component = state.components.get(&component_id);

    if component.is_none() {
        return rsx!(div { "Component not found" });
    }
    let component = component.unwrap();
    let style_pairs: HashMap<String, String> = component.styles.clone();

    rsx!(
        div { 
            class: "styles-editor",
            for (key, value) in style_pairs {
                {
                    let key_c = key.clone();
                    let key_c1 = key.clone();
                    rsx!(
                        div { 
                            input {
                                value: "{key}",
                                oninput: move |e| {
                                    let mut state = EDITOR_STATE.write();
                                    if let Some(comp) = state.components.get_mut(&component_id) {
                                        if let Some(v) = comp.styles.remove(&key_c) {
                                            comp.styles.insert(e.value(), v);
                                        }
                                    }
                                }
                            }
                            input {
                                value: "{value}",
                                oninput: move |e| {
                                    let mut state = EDITOR_STATE.write();
                                    if let Some(comp) = state.components.get_mut(&component_id) {
                                        comp.styles.insert(key.clone(), e.value());
                                    }
                                }
                            }
                            button {
                                onclick: move |_| {
                                    let mut state = EDITOR_STATE.write();
                                    if let Some(comp) = state.components.get_mut(&component_id) {
                                        comp.styles.remove(&key_c1);
                                    }
                                },
                                "X"
                            }

                        }
                    )
                }
            }

            button {
                onclick: move |_| {
                    let mut state = EDITOR_STATE.write();
                    if let Some(comp) = state.components.get_mut(&component_id) {
                        let mut new_key = "new-property".to_string();
                        let mut counter = 1;
                        while comp.styles.contains_key(&new_key) {
                            new_key = format!("new-property-{}", counter);
                            counter += 1;
                        }
                        comp.styles.insert(new_key, "".to_string());
                    }
                },
                "Add style"
            }
        }
    )
}
