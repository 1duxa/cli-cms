use dioxus::prelude::*;
use super::styles_editor::StyleInput;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum ComponentType {
    Container,
    Heading,
    Paragraph,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub id: usize,
    pub component_type: ComponentType,
    pub children: Vec<usize>, 
    pub styles: HashMap<String, String>,
    pub content: String,
    pub x: f64, 
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorMode {
    Editor,
    Preview,
}

#[derive(Clone, Debug)]
pub struct EditorState {
    pub components: HashMap<usize, Component>,
    pub next_id: usize,
    pub selected_id: Option<usize>,
    pub dragging_id: Option<usize>,
    pub drag_offset_x: f64,
    pub drag_offset_y: f64,
    pub mode: EditorMode,
    pub hovering_container_id: Option<usize>, // For connection UI
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
            next_id: 0,
            selected_id: None,
            dragging_id: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            mode: EditorMode::Editor,
            hovering_container_id: None,
        }
    }
}

pub static EDITOR_STATE: GlobalSignal<EditorState> = Signal::global(EditorState::default);

#[component]
pub fn VisualEditor() -> Element {
    let state = EDITOR_STATE.read();
    let editor_bg = if state.mode == EditorMode::Editor { "var(--color-primary)" } else { "var(--color-secondary)" };
    let preview_bg = if state.mode == EditorMode::Preview { "var(--color-primary)" } else { "var(--color-secondary)" };
    
    rsx! {
        div {
            class: "visual-editor",
            style: "display: flex; height: 100vh; font-family: system-ui;",
            
            div {
                class: "toolbox",
                h2 { style: "margin: 0 0 16px 0; font-size: 18px;", "Components" }
                
                div {
                    class: "mode-toggle",
                    style: "margin-bottom: 16px; display: flex; gap: 8px;",
                    button {
                        onclick: move |_| set_mode(EditorMode::Editor),
                        style: "background: {editor_bg};",
                        "Editor"
                    }
                    button {
                        onclick: move |_| set_mode(EditorMode::Preview),
                        style: "background: {preview_bg};",
                        "Preview"
                    }
                }
                
                if state.mode == EditorMode::Editor {
                    div {
                        class: "component-buttons",
                        style: "display: flex; flex-direction: column; gap: 8px;",
                        
                        button {
                            onclick: move |_| add_component(ComponentType::Container),
                            "Container"
                        }
                        button {
                            onclick: move |_| add_component(ComponentType::Heading),
                            "Heading"
                        }
                        button {
                            onclick: move |_| add_component(ComponentType::Paragraph),
                            "Paragraph"
                        }
                    }
                    
                    div { style: "margin-top: 24px;",
                        h3 { style: "margin: 0 0 8px 0; font-size: 14px;", "Instructions" }
                        p { style: "font-size: 12px; color: #666; line-height: 1.4;",
                            "Click boxes to select"
                            br {}
                            "Drag boxes to move"
                            br {}
                            " Containers can have children"
                            br {}
                            " Connect with arrows"
                        }
                    }
                }
            }
            
            // Center - Canvas
            div {
                class: "canvas-wrapper",
                style: "flex: 1; background: #f0f0f0; overflow: hidden; position: relative;",
                
                if state.mode == EditorMode::Editor {
                    Canvas {}
                } else {
                    PreviewCanvas {}
                }
            }
            
            // Right sidebar - Properties
            if state.mode == EditorMode::Editor {
                div {
                    class: "properties",
                    PropertiesPanel {}
                }
            }
        }
    }
}

#[component]
fn Canvas() -> Element {
    let state = EDITOR_STATE.read();
    
    rsx! {
        div {
            class: "canvas",
            style: "width: 100%; height: 100%; position: relative;",
            onmouseup: move |_| stop_dragging(),
            onmousemove: move |e| handle_drag(e.page_coordinates().x, e.page_coordinates().y),
            
            // Draw connection arrows
            svg {
                style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none;",
                for (id, component) in state.components.iter() {
                    for child_id in component.children.iter() {
                        if let Some(child) = state.components.get(child_id) {
                            {
                                let x1 = component.x + 100.0; // center of box
                                let y1 = component.y + 40.0;
                                let x2 = child.x + 100.0;
                                let y2 = child.y + 40.0;
                                rsx! {
                                    line {
                                        x1: "{x1}",
                                        y1: "{y1}",
                                        x2: "{x2}",
                                        y2: "{y2}",
                                        stroke: "#666",
                                        stroke_width: "2",
                                        marker_end: "url(#arrowhead)",
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Arrow marker definition
                defs {
                    marker {
                        id: "arrowhead",
                        marker_width: "10",
                        marker_height: "10",
                        ref_x: "9",
                        ref_y: "3",
                        orient: "auto",
                        polygon {
                            points: "0 0, 10 3, 0 6",
                            fill: "#666"
                        }
                    }
                }
            }
            
            // Draw component boxes
            for (id, component) in state.components.iter() {
                ComponentBox { component_id: *id }
            }
        }
    }
}

#[component]
fn ComponentBox(component_id: usize) -> Element {
    let state = EDITOR_STATE.read();
    let (component_type, component_content, component_children_len, component_x, component_y) = if let Some(c) = state.components.get(&component_id) {
        (c.component_type.clone(), &c.content, c.children.len(), c.x, c.y)
    } else {
        panic!("Not found")
    };
    let is_selected = state.selected_id == Some(component_id);
    let is_hovering = state.hovering_container_id == Some(component_id);
    
    let (type_name, type_color) = match component_type {
        ComponentType::Container => ("Container", "#4CAF50"),
        ComponentType::Heading => ("Heading", "#2196F3"),
        ComponentType::Paragraph => ("Paragraph", "#FF9800"),
    };
    
    let border_color = if is_selected { 
        "#f44336" 
    } else if is_hovering && component_type == ComponentType::Container {
        "#9C27B0"
    } else { 
        "#333" 
    };
    
    let border_width = if is_selected || is_hovering { "3px" } else { "2px" };
    let box_shadow = if is_hovering { 
        "0 4px 12px rgba(156, 39, 176, 0.4)" 
    } else { 
        "0 2px 8px rgba(0,0,0,0.2)" 
    };
    
    rsx! {
        div {
            class: "component-box",
            style: "
                position: absolute;
                left: {component_x}px;
                top: {component_y}px;
                width: 200px;
                background: {type_color};
                border: {border_width} solid {border_color};
                border-radius: 8px;
                padding: 12px;
                cursor: grab;
                user-select: none;
                box-shadow: {box_shadow};
            ",
            onmousedown: move |e| {
                e.stop_propagation();
                start_dragging(component_id, e.page_coordinates().x, e.page_coordinates().y);
            },
            onclick: move |e| {
                e.stop_propagation();
                select_component(component_id);
            },
            onmouseenter: move |_| {
                if component_type == ComponentType::Container {
                    set_hovering_container(Some(component_id));
                }
            },
            onmouseleave: move |_| {
                set_hovering_container(None);
            },
            
            div {
                style: "font-weight: bold; color: white; font-size: 14px; margin-bottom: 4px;",
                "{type_name} #{component_id}"
            }
            
            if component_type == ComponentType::Container {
                div {
                    style: "color: rgba(255,255,255,0.8); font-size: 12px;",
                    "Children: {component_children_len}"
                }
                if is_hovering {
                    div {
                        style: "margin-top: 8px; padding: 4px; background: rgba(255,255,255,0.2); 
                                border-radius: 4px; text-align: center; font-size: 11px; color: white;",
                        "🔗 Click to connect"
                    }
                }
            } else if !component_content.is_empty() {
                div {
                    style: "color: rgba(255,255,255,0.9); font-size: 12px; 
                            overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    "{component_content}"
                }
            }
        }
    }
}

#[component]
fn PropertiesPanel() -> Element {
    let state = EDITOR_STATE.read();
    
    let Some(selected_id) = state.selected_id else {
        return rsx! {
            div { 
                style: "color: slate; text-align: center; padding: 32px;",
                "Select a component"
            }
        };
    };
    
    let Some(component) = state.components.get(&selected_id) else {
        return rsx! { div { "Component not found" } };
    };
    
    rsx! {
        div { class: "properties-panel",
            if component.component_type != ComponentType::Container {
                div { 
                    style: "display:flex;flex-direction:column;padding-inline:12px;",
                    h1 { style: "color:slate;text-align:center; margin: 24px 0 12px 0; font-size: 18px;", "Content" }

                    input {
                        r#type: "text",
                        value: "{component.content}",
                        oninput: move |e| update_content(selected_id, e.value()),
                    }
                }
            }
            
            h1 { style: "color:slate;text-align:center; margin: 24px 0 12px 0; font-size: 18px;", "Styles" }
            
            StyleInput { component_id: selected_id }
   
            if component.component_type == ComponentType::Container {
                h4 { style: "margin: 24px 0 12px 12px; font-size: 14px;", "Children" }
                div { style: "font-size: 12px; color: #666;margin: 12px 0 0 12px;",
                    if component.children.is_empty() {
                        "No children yet"
                    } else {
                        "Children: {component.children.len()}"
                    }
                }
                button {
                    onclick: move |_| add_child_to_container(selected_id),
                    style: "margin: 12px 0 0 12px; padding: 6px 12px; cursor: pointer;",
                    "Add Child Connection"
                }
            }
            
            div { style: "margin-top: 24px; padding-inline: 12px",
                button {
                    onclick: move |_| delete_component(selected_id),
                    style: "width: 100%; padding: 8px; cursor: pointer; 
                            background: #f44336; color: white; border: none; border-radius: 4px;",
                    "Delete Component"
                }
            }
        }
    }
}

#[component]
fn PreviewCanvas() -> Element {
    let state = EDITOR_STATE.read();
    
    rsx! {
        div {
            style: "width: 100%; height: 100%; background: white; padding: 32px; overflow-y: auto;",
            
            for (id, component) in state.components.iter().filter(|(_, c)| {
                !state.components.values().any(|comp| comp.children.contains(&c.id))
            }) {
                PreviewComponent { component_id: *id }
            }
        }
    }
}

#[component]
fn PreviewComponent(component_id: usize) -> Element {
    let state = EDITOR_STATE.read();
    let component = state.components.get(&component_id).unwrap();
    
    let style_str = component.styles.iter()
        .map(|(k, v)| format!("{}: {};", k, v))
        .collect::<Vec<_>>()
        .join(" ");
    
    match component.component_type {
        ComponentType::Container => rsx! {
            div { style: "{style_str}",
                for child_id in component.children.iter() {
                    PreviewComponent { component_id: *child_id }
                }
            }
        },
        ComponentType::Heading => rsx! {
            h1 { style: "{style_str}", "{component.content}" }
        },
        ComponentType::Paragraph => rsx! {
            p { style: "{style_str}", "{component.content}" }
        },
    }
}

fn add_component(component_type: ComponentType) {
    let mut state = EDITOR_STATE.write();
    let id = state.next_id;
    state.next_id += 1;
    
    let default_content = match component_type {
        ComponentType::Heading => "Heading Text".to_string(),
        ComponentType::Paragraph => "Paragraph text".to_string(),
        ComponentType::Container => String::new(),
    };
    
    let component = Component {
        id,
        component_type,
        children: Vec::new(),
        styles: HashMap::new(),
        content: default_content,
        x: 50.0 + (id as f64 * 20.0),
        y: 50.0 + (id as f64 * 20.0),
    };
    
    state.components.insert(id, component);
    state.selected_id = Some(id);
}

fn select_component(id: usize) {
    EDITOR_STATE.write().selected_id = Some(id);
}

fn start_dragging(id: usize, mouse_x: f64, mouse_y: f64) {
    let mut state = EDITOR_STATE.write();
    
    let (offset_x, offset_y) = if let Some(component) = state.components.get(&id) {
        (mouse_x - component.x, mouse_y - component.y)
    } else {
        return;
    };
    
    state.dragging_id = Some(id);
    state.drag_offset_x = offset_x;
    state.drag_offset_y = offset_y;
    state.selected_id = Some(id);
}

fn handle_drag(mouse_x: f64, mouse_y: f64) {
    let mut state = EDITOR_STATE.write();
    if let Some(id) = state.dragging_id {
        let drag_x = state.drag_offset_x;
        let drag_y = state.drag_offset_y;
        if let Some(component) = state.components.get_mut(&id) {
            component.x = mouse_x - drag_x;
            component.y = mouse_y - drag_y;
        }
    }
}

fn stop_dragging() {
    EDITOR_STATE.write().dragging_id = None;
}

fn delete_component(id: usize) {
    let mut state = EDITOR_STATE.write();
    
    for component in state.components.values_mut() {
        component.children.retain(|&child_id| child_id != id);
    }
    
    state.components.remove(&id);
    
    if state.selected_id == Some(id) {
        state.selected_id = None;
    }
}

fn update_content(component_id: usize, content: String) {
    let mut state = EDITOR_STATE.write();
    if let Some(component) = state.components.get_mut(&component_id) {
        component.content = content;
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

fn add_child_to_container(container_id: usize) {
    let mut state = EDITOR_STATE.write();
    
    if let Some(&available_id) = state.components.keys().find(|&&id| 
            id != container_id && !state.components.get(&container_id).unwrap().children.contains(&id)) {
        if let Some(container) = state.components.get_mut(&container_id) {
            container.children.push(available_id);
        }
    }
}

fn set_mode(mode: EditorMode) {
    EDITOR_STATE.write().mode = mode;
}

fn set_hovering_container(id: Option<usize>) {
    EDITOR_STATE.write().hovering_container_id = id;
}
