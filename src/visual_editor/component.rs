use dioxus::prelude::*;
use super::styles_editor::StyleInput;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

static WINDOW_MOUSEUP_INSTALLED: AtomicBool = AtomicBool::new(false);

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

    // Connection/drawing state
    pub connecting_from: Option<usize>,
    pub connecting_mouse_x: f64,
    pub connecting_mouse_y: f64,
    pub connecting_hover_target_id: Option<usize>,

    // Suppress clicks that occur immediately after a drag
    pub just_dragged: bool,
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

            connecting_from: None,
            connecting_mouse_x: 0.0,
            connecting_mouse_y: 0.0,
            connecting_hover_target_id: None,

            just_dragged: false,
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
                id: "canvas",
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

    // Compute preview line coordinates outside of rsx! to avoid complex let bindings inside the macro
    let preview_line_coords = if let Some(from_id) = state.connecting_from {
        if let Some(from_comp) = state.components.get(&from_id) {
            let start_cx = from_comp.x + 100.0;
            let start_cy = from_comp.y + 40.0;

            // end point snaps to target edge when hovering a valid component, otherwise follows mouse
            let (end_x, end_y) = if let Some(target_id) = state.connecting_hover_target_id {
                if let Some(target) = state.components.get(&target_id) {
                    rect_edge_point_towards(start_cx, start_cy, target.x, target.y, 200.0, 80.0)
                } else {
                    (state.connecting_mouse_x, state.connecting_mouse_y)
                }
            } else {
                (state.connecting_mouse_x, state.connecting_mouse_y)
            };

            // start point should snap to parent edge towards the end point
            let (sx, sy) = rect_edge_point_towards(end_x, end_y, from_comp.x, from_comp.y, 200.0, 80.0);
            Some((sx, sy, end_x, end_y))
        } else {
            None
        }
    } else {
        None
    };

    rsx! {
        div {
            class: "canvas",
            style: "width: 100%; height: 100%; position: relative;",
            // Cancel connecting on background click
            onmousedown: move |_| {
                if EDITOR_STATE.read().connecting_from.is_some() {
                    stop_connecting();
                }
            },
            onmouseup: move |_| stop_dragging(),
            // update dragging & connecting preview
            onmousemove: move |e| handle_mouse_move(e.page_coordinates().x, e.page_coordinates().y),

            // Draw connection arrows
            svg {
                style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none;",
                for (id, component) in state.components.iter() {
                    for child_id in component.children.iter() {
                        if let Some(child) = state.components.get(child_id) {
                            {
                                // Compute snapped endpoints so arrows touch the child edge (and parent edge)
                                let parent_cx = component.x + 100.0;
                                let parent_cy = component.y + 40.0;

                                let (x1, y1) = rect_edge_point_towards(child.x + 100.0, child.y + 40.0, component.x, component.y, 200.0, 80.0); // parent edge
                                let (x2, y2) = rect_edge_point_towards(parent_cx, parent_cy, child.x, child.y, 200.0, 80.0); // child edge

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

                // Preview connecting line (while the user is drawing a new connection)
                if let Some((sx, sy, end_x, end_y)) = preview_line_coords {
                    {
                        rsx! {
                            line {
                                x1: "{sx}",
                                y1: "{sy}",
                                x2: "{end_x}",
                                y2: "{end_y}",
                                stroke: "#f44336",
                                stroke_width: "2",
                                stroke_dasharray: "6 4",
                                marker_end: "url(#arrowhead)",
                            }
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
    let is_connect_target = state.connecting_hover_target_id == Some(component_id);

    // Precompute whether this is the container that is currently initiating a connection
    let is_connecting_from_here = state.connecting_from == Some(component_id);

    let (type_name, type_color) = match component_type {
        ComponentType::Container => ("Container", "#4CAF50"),
        ComponentType::Heading => ("Heading", "#2196F3"),
        ComponentType::Paragraph => ("Paragraph", "#FF9800"),
    };

    let border_color = if is_selected {
        "#f44336"
    } else if is_connect_target {
        "#FF5722"
    } else if is_hovering && component_type == ComponentType::Container {
        "#9C27B0"
    } else { 
        "#333" 
    };

    let border_width = if is_selected || is_hovering || is_connect_target { "3px" } else { "2px" };
    let box_shadow = if is_hovering || is_connect_target {
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
            // If connecting, clicking on a component finishes the connection, otherwise starts dragging
            onmousedown: move |e| {
                e.stop_propagation();
                if EDITOR_STATE.read().connecting_from.is_some() {
                    // don't start dragging while connecting
                } else {
                    start_dragging(component_id, e.page_coordinates().x, e.page_coordinates().y);
                }
            },
            onclick: move |e| {
                e.stop_propagation();

                // Diagnostic log for clicks
                #[cfg(target_arch = "wasm32")]
                {
                    let conn = { let s = EDITOR_STATE.read(); s.connecting_from };
                    let jd = { let s = EDITOR_STATE.read(); s.just_dragged };
                    web_sys::console::log_1(&format!("onclick: component {} clicked (connecting_from={:?}, just_dragged={})", component_id, conn, jd).into());
                }

                // If currently connecting, complete the connection even if just_dragged was recently set
                if { let s = EDITOR_STATE.read(); s.connecting_from.is_some() } {
                    // If there was a leftover just_dragged flag, clear it so the click isn't ignored
                    if { let s = EDITOR_STATE.read(); s.just_dragged } {
                        let mut s = EDITOR_STATE.write();
                        s.just_dragged = false;
                    }

                    if let Some(from_id) = { let s = EDITOR_STATE.read(); s.connecting_from } {
                        if from_id != component_id {
                            #[cfg(target_arch = "wasm32")]
                            { web_sys::console::log_1(&format!("onclick: completing connection {} -> {}", from_id, component_id).into()); }
                            complete_connection(from_id, component_id);
                        }
                        stop_connecting();
                    }

                    return;
                }

                // Not connecting: handle standard click (ignore clicks immediately after dragging)
                if { let s = EDITOR_STATE.read(); s.just_dragged } {
                    let mut s = EDITOR_STATE.write();
                    s.just_dragged = false;
                    return;
                }

                // Normal selection
                select_component(component_id);
            },
            onmouseup: move |e| {
                e.stop_propagation();

                #[cfg(target_arch = "wasm32")]
                {
                    let conn = { let s = EDITOR_STATE.read(); s.connecting_from };
                    web_sys::console::log_1(&format!("onmouseup: component {} (connecting_from={:?})", component_id, conn).into());
                }

                if { let s = EDITOR_STATE.read(); s.connecting_from.is_some() } {
                    // If there was a leftover just_dragged flag, clear it
                    if { let s = EDITOR_STATE.read(); s.just_dragged } {
                        let mut s = EDITOR_STATE.write();
                        s.just_dragged = false;
                    }

                    if let Some(from_id) = { let s = EDITOR_STATE.read(); s.connecting_from } {
                        if from_id != component_id {
                            #[cfg(target_arch = "wasm32")]
                            { web_sys::console::log_1(&format!("onmouseup: completing connection {} -> {}", from_id, component_id).into()); }
                            complete_connection(from_id, component_id);
                        }
                        stop_connecting();
                    }
                }
            },
            onmouseenter: move |_| {
                if component_type == ComponentType::Container {
                    set_hovering_container(Some(component_id));
                }
                // if we're connecting, mark this as potential target
                if EDITOR_STATE.read().connecting_from.is_some() && EDITOR_STATE.read().connecting_from != Some(component_id) {
                    set_connecting_hover_target(Some(component_id));
                }
            },
            onmouseleave: move |_| {
                set_hovering_container(None);
                set_connecting_hover_target(None);
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
                                border-radius: 4px; text-align: center; font-size: 11px; color: white; cursor: pointer;",
                        onclick: move |e| { e.stop_propagation(); start_connecting(component_id); },
                        if is_connecting_from_here { "🔗 Connecting..." } else { "🔗 Click to connect" }
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
            style: "width: 100%; height: 100%; background: white; overflow-y: auto;",
            
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
    // Convert to local coordinates
    let (local_x, local_y) = page_to_local(mouse_x, mouse_y);

    // compute offsets without holding a write lock
    let (offset_x, offset_y) = if let Some(component) = EDITOR_STATE.read().components.get(&id) {
        (local_x - component.x, local_y - component.y)
    } else {
        return;
    };

    let mut state = EDITOR_STATE.write();
    state.dragging_id = Some(id);
    state.drag_offset_x = offset_x;
    state.drag_offset_y = offset_y;
    state.selected_id = Some(id);

    // Attach a global window-level mouseup listener once so releasing outside the canvas also stops dragging
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if !WINDOW_MOUSEUP_INSTALLED.load(Ordering::SeqCst) {
            if let Some(window) = web_sys::window() {
                let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move |_: web_sys::Event| {
                    stop_dragging();
                }) as Box<dyn FnMut(web_sys::Event)>);
                let _ = window.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref());
                // keep it alive permanently (single global handler)
                closure.forget();
                WINDOW_MOUSEUP_INSTALLED.store(true, Ordering::SeqCst);
            }
        }
    }
}

// Convert page coordinates to coordinates local to the canvas element (id="canvas").
fn page_to_local(page_x: f64, page_y: f64) -> (f64, f64) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(elem) = document.get_element_by_id("canvas") {
                    let rect = elem.get_bounding_client_rect();
                    // rect.left/top are relative to the viewport; page coordinates include scroll offset
                    let scroll_x = window.page_x_offset().unwrap_or(0.0);
                    let scroll_y = window.page_y_offset().unwrap_or(0.0);
                    let elem_left_page = rect.left() + scroll_x;
                    let elem_top_page = rect.top() + scroll_y;
                    return (page_x - elem_left_page, page_y - elem_top_page);
                }
            }
        }
        (page_x, page_y)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Non-web targets: assume coordinates are already local
        (page_x, page_y)
    }
}

// Updated to also handle connecting mouse movement & hover detection, using local coordinates and separating reads/writes
fn handle_mouse_move(page_mouse_x: f64, page_mouse_y: f64) {
    let (mouse_x, mouse_y) = page_to_local(page_mouse_x, page_mouse_y);

    // Handle dragging by reading minimal state first, then performing a focused write
    if let Some(id) = { let s = EDITOR_STATE.read(); s.dragging_id } {
        let (drag_x, drag_y) = { let s = EDITOR_STATE.read(); (s.drag_offset_x, s.drag_offset_y) };
        let new_x = mouse_x - drag_x;
        let new_y = mouse_y - drag_y;
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!("handle_mouse_move: attempting write to move id={} to {} {}", id, new_x, new_y).into());
        }
        let mut s = EDITOR_STATE.write();
        if let Some(component) = s.components.get_mut(&id) {
            component.x = new_x;
            component.y = new_y;
        }
    }

    // Update connecting preview position and hovered target
    if { let s = EDITOR_STATE.read(); s.connecting_from.is_some() } {
        // compute hovered target under mouse using a read lock
        let hovered = { 
            let s = EDITOR_STATE.read();
            s.components.iter().find_map(|(&id, comp)| {
                if s.connecting_from == Some(id) { return None; }
                let left = comp.x;
                let right = comp.x + 200.0;
                let top = comp.y;
                let bottom = comp.y + 80.0;
                if mouse_x >= left && mouse_x <= right && mouse_y >= top && mouse_y <= bottom {
                    Some(id)
                } else { None }
            })
        };

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!("handle_mouse_move: updating connecting mouse to {} {}, hovered={:?}", mouse_x, mouse_y, hovered).into());
        }

        let mut s = EDITOR_STATE.write();
        s.connecting_mouse_x = mouse_x;
        s.connecting_mouse_y = mouse_y;
        s.connecting_hover_target_id = hovered;
    }
}

fn stop_dragging() {
    // Try to clear immediately; if there's a borrow conflict, fall back to scheduling on next tick
    let immediate_ok = std::panic::catch_unwind(|| {
        let mut s = EDITOR_STATE.write();
        s.dragging_id = None;
        s.just_dragged = true;
    }).is_ok();

    if immediate_ok {
        return;
    }

    // Schedule clearing dragging state on the next tick in web to avoid borrow races with click handlers
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            // clone window for use inside closures so we don't move `window`
            let window_clone = window.clone();
            let attempt = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                #[cfg(target_arch = "wasm32")]
                {
                    web_sys::console::log_1(&"stop_dragging: attempt write".into());
                }

                // Try to write; if it panics because the signal is borrowed, reschedule another attempt
                let ok = std::panic::catch_unwind(|| {
                    let mut s = EDITOR_STATE.write();
                    s.dragging_id = None;
                    s.just_dragged = true;
                });

                if ok.is_err() {
                    // reschedule another attempt on the next tick
                    let window_retry = window_clone.clone();
                    let retry = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                        let _ = std::panic::catch_unwind(|| {
                            let mut s = EDITOR_STATE.write();
                            s.dragging_id = None;
                            s.just_dragged = true;
                        });
                    }) as Box<dyn FnMut()>);
                    let _ = window_retry.set_timeout_with_callback_and_timeout_and_arguments_0(retry.as_ref().unchecked_ref(), 0);
                    retry.forget();
                }
            }) as Box<dyn FnMut()>);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(attempt.as_ref().unchecked_ref(), 0);
            attempt.forget();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut state = EDITOR_STATE.write();
        state.dragging_id = None;
        state.just_dragged = true;
    }
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

// Add a child by id (used when completing a manual connection)
fn complete_connection(from_id: usize, to_id: usize) {
    let mut state = EDITOR_STATE.write();
    if let Some(from) = state.components.get_mut(&from_id) {
        if from.component_type != ComponentType::Container {
            return; // only containers can have children
        }
        if !from.children.contains(&to_id) && to_id != from_id {
            from.children.push(to_id);
            state.selected_id = Some(to_id);

            #[cfg(target_arch = "wasm32")]
            {
                web_sys::console::log_1(&format!("complete_connection: {} -> {}", from_id, to_id).into());
            }
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

fn set_connecting_hover_target(id: Option<usize>) {
    EDITOR_STATE.write().connecting_hover_target_id = id;
}

fn start_connecting(id: usize) {
    // Read component coordinates first under a read lock to avoid overlapping borrows
    let (comp_x, comp_y) = {
        let state_read = EDITOR_STATE.read();
        if let Some(comp) = state_read.components.get(&id) {
            (comp.x, comp.y)
        } else {
            (0.0, 0.0)
        }
    };

    let mut state = EDITOR_STATE.write();
    state.connecting_from = Some(id);
    state.connecting_mouse_x = comp_x + 100.0;
    state.connecting_mouse_y = comp_y + 40.0;
}

fn stop_connecting() {
    let mut state = EDITOR_STATE.write();
    state.connecting_from = None;
    state.connecting_hover_target_id = None;
}

// Calculate the point on the perimeter of an axis-aligned rectangle (rect_x, rect_y, rect_w, rect_h)
// that lies on the line from the rect's center toward (source_x, source_y).
fn rect_edge_point_towards(source_x: f64, source_y: f64, rect_x: f64, rect_y: f64, rect_w: f64, rect_h: f64) -> (f64, f64) {
    let cx = rect_x + rect_w / 2.0;
    let cy = rect_y + rect_h / 2.0;
    let vx = source_x - cx;
    let vy = source_y - cy;

    if vx == 0.0 && vy == 0.0 {
        return (cx, cy);
    }

    let hw = rect_w / 2.0;
    let hh = rect_h / 2.0;
    let mut s = f64::INFINITY;
    if vx.abs() > 0.0 { s = s.min(hw / vx.abs()); }
    if vy.abs() > 0.0 { s = s.min(hh / vy.abs()); }
    if !s.is_finite() {
        return (cx, cy);
    }

    (cx + vx * s, cy + vy * s)
}

fn schedule_task<F: 'static + FnOnce()>(f: F) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            let mut opt = Some(f);
            let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                if let Some(func) = opt.take() {
                    func();
                }
            }) as Box<dyn FnMut()>);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 0);
            closure.forget();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // non-web targets: run immediately
        f();
    }
}
