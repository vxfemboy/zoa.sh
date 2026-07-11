// This file is only compiled for WASM target
#[cfg(target_arch = "wasm32")]
mod wasm {
    use js_sys::Math;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, Element, HtmlElement, HtmlPreElement, MouseEvent};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    fn debug_enabled() -> bool {
        if let Some(win) = window() {
            if let Ok(val) = js_sys::Reflect::get(&win.into(), &JsValue::from_str("ZOA_DEBUG")) {
                return val.as_bool().unwrap_or(false);
            }
        }
        false
    }

    macro_rules! console_log {
        ($($t:tt)*) => ({ if debug_enabled() { log(&format_args!($($t)*).to_string()) } })
    }

    const NEKO_SPEED: f64 = 10.0; // Movement speed per tick (~matches current animation cadence)

    #[wasm_bindgen]
    pub struct AsciiCat {
        cat_element: HtmlPreElement,

        // Position (like oneko.js)
        neko_pos_x: f64,
        neko_pos_y: f64,
        mouse_pos_x: f64,
        mouse_pos_y: f64,

        // Animation state (like oneko.js)
        frame_count: usize,
        idle_time: usize,
        idle_animation: Option<String>,
        idle_animation_frame: usize,

        // Animation frames cache
        frames_cache: std::collections::HashMap<String, String>,
    }

    #[wasm_bindgen]
    impl AsciiCat {
        #[wasm_bindgen(constructor)]
        pub fn new(animations: &JsValue) -> Result<AsciiCat, JsValue> {
            console_log!("Starting AsciiCat constructor (oneko.js style)");

            let window = window().ok_or("No window")?;
            let document = window.document().ok_or("No document")?;

            // Create cat element
            let cat_element = document
                .create_element("pre")?
                .dyn_into::<HtmlPreElement>()?;

            cat_element.set_id("ascii-cat");

            // Set initial styles (oneko.js inspired) - 3x smaller
            cat_element.set_attribute(
                "style",
                "position: fixed; \
                 pointer-events: none; \
                 z-index: 99999; \
                 font-size: 3px; \
                 line-height: 1; \
                 white-space: pre; \
                 color: #fff; \
                 text-shadow: 0 0 2px rgba(255,255,255,0.8); \
                 filter: drop-shadow(0 0 1px rgba(255,255,255,0.5));",
            )?;

            // Append to body
            document
                .body()
                .ok_or("No body")?
                .append_child(&cat_element)?;

            // Parse animations from JavaScript object
            let mut frames_cache = std::collections::HashMap::new();

            if let Ok(animations_obj) = animations.clone().dyn_into::<js_sys::Object>() {
                let keys = js_sys::Object::keys(&animations_obj);
                console_log!("Loading {} animations", keys.length());

                for i in 0..keys.length() {
                    if let Some(key) = keys.get(i).as_string() {
                        if let Ok(value_js) =
                            js_sys::Reflect::get(&animations_obj, &JsValue::from_str(&key))
                        {
                            if let Some(value) = value_js.as_string() {
                                frames_cache.insert(key.clone(), value);
                                console_log!("Loaded: {}", key);
                            }
                        }
                    }
                }
            }

            console_log!("Loaded {} animations into cache", frames_cache.len());

            let mut cat = AsciiCat {
                cat_element,
                neko_pos_x: 32.0,
                neko_pos_y: 32.0,
                mouse_pos_x: 0.0,
                mouse_pos_y: 0.0,
                frame_count: 0,
                idle_time: 0,
                idle_animation: None,
                idle_animation_frame: 0,
                frames_cache,
            };

            // Set initial sprite
            cat.set_sprite("still")?;

            Ok(cat)
        }

        fn set_sprite(&mut self, name: &str) -> Result<(), JsValue> {
            if let Some(sprite) = self.frames_cache.get(name) {
                self.cat_element.set_inner_html(sprite);
            }
            Ok(())
        }

        fn get_direction_from_diff(diff_x: f64, diff_y: f64, distance: f64) -> String {
            let mut direction = String::new();

            if diff_y / distance > 0.5 {
                direction.push('N');
            } else if diff_y / distance < -0.5 {
                direction.push('S');
            }

            if diff_x / distance > 0.5 {
                direction.push('W');
            } else if diff_x / distance < -0.5 {
                direction.push('E');
            }

            if direction.is_empty() {
                direction = "still".to_string();
            }

            direction
        }

        fn reset_idle_animation(&mut self) {
            self.idle_animation = None;
            self.idle_animation_frame = 0;
        }

        fn idle(&mut self) -> Result<(), JsValue> {
            self.idle_time += 1;

            // Every ~20 seconds (like oneko.js)
            if self.idle_time > 10
                && (Math::random() * 200.0).floor() == 0.0
                && self.idle_animation.is_none()
            {
                let mut available_animations =
                    vec!["sleep".to_string(), "itch".to_string(), "wash".to_string()];

                let window = window().ok_or("No window")?;
                let inner_width = window.inner_width()?.as_f64().unwrap_or(800.0);
                let inner_height = window.inner_height()?.as_f64().unwrap_or(600.0);

                if self.neko_pos_x < 32.0 {
                    available_animations.push("wscratch".to_string());
                }
                if self.neko_pos_y < 32.0 {
                    available_animations.push("nscratch".to_string());
                }
                if self.neko_pos_x > inner_width - 32.0 {
                    available_animations.push("escratch".to_string());
                }
                if self.neko_pos_y > inner_height - 32.0 {
                    available_animations.push("sscratch".to_string());
                }

                let idx = (Math::random() * available_animations.len() as f64).floor() as usize;
                self.idle_animation = Some(available_animations[idx].clone());
            }

            if let Some(ref anim) = self.idle_animation.clone() {
                match anim.as_str() {
                    "sleep" => {
                        if self.idle_animation_frame < 8 {
                            self.set_sprite("yawn")?;
                        } else {
                            let frame_name =
                                format!("sleep{}", (self.idle_animation_frame / 4) % 2 + 1);
                            self.set_sprite(&frame_name)?;
                        }
                        if self.idle_animation_frame > 192 {
                            self.reset_idle_animation();
                        }
                    }
                    "wash" => {
                        self.set_sprite("wash")?;
                        if self.idle_animation_frame > 9 {
                            self.reset_idle_animation();
                        }
                    }
                    "nscratch" | "escratch" | "sscratch" | "wscratch" | "itch" => {
                        let frame_name = format!("{}{}", anim, (self.idle_animation_frame % 2) + 1);
                        self.set_sprite(&frame_name)?;
                        if self.idle_animation_frame > 9 {
                            self.reset_idle_animation();
                        }
                    }
                    _ => {
                        self.set_sprite("still")?;
                        return Ok(());
                    }
                }
                self.idle_animation_frame += 1;
            } else {
                self.set_sprite("still")?;
            }

            Ok(())
        }

        pub fn update_mouse(&mut self, x: f64, y: f64) {
            self.mouse_pos_x = x;
            self.mouse_pos_y = y;
        }

        pub fn update(&mut self) -> Result<(), JsValue> {
            self.frame_count += 1;

            let diff_x = self.neko_pos_x - self.mouse_pos_x;
            let diff_y = self.neko_pos_y - self.mouse_pos_y;
            let distance = Math::sqrt(diff_x * diff_x + diff_y * diff_y);

            // If close to mouse or very close, idle
            if distance < NEKO_SPEED || distance < 48.0 {
                self.idle()?;
                self.update_position()?;
                return Ok(());
            }

            self.idle_animation = None;
            self.idle_animation_frame = 0;

            // Alert state (like oneko.js)
            if self.idle_time > 1 {
                self.set_sprite("alert")?;
                self.idle_time = self.idle_time.min(7);
                self.idle_time -= 1;
                self.update_position()?;
                return Ok(());
            }

            // Get direction and set sprite
            let direction = Self::get_direction_from_diff(diff_x, diff_y, distance);

            // Slow down animation cycling - change frame every 10 updates instead of every update
            let anim_frame = ((self.frame_count / 10) % 2) + 1;

            let frame_name = format!(
                "{}{}",
                direction.to_lowercase(),
                if direction == "still" {
                    String::new()
                } else {
                    format!("run{}", anim_frame)
                }
            );

            self.set_sprite(&frame_name)?;

            // Move towards mouse
            self.neko_pos_x -= (diff_x / distance) * NEKO_SPEED;
            self.neko_pos_y -= (diff_y / distance) * NEKO_SPEED;

            // Clamp to window bounds
            let window = window().ok_or("No window")?;
            let inner_width = window.inner_width()?.as_f64().unwrap_or(800.0);
            let inner_height = window.inner_height()?.as_f64().unwrap_or(600.0);

            self.neko_pos_x = self.neko_pos_x.max(16.0).min(inner_width - 16.0);
            self.neko_pos_y = self.neko_pos_y.max(16.0).min(inner_height - 16.0);

            self.update_position()?;
            Ok(())
        }

        fn update_position(&mut self) -> Result<(), JsValue> {
            self.cat_element.set_attribute(
                "style",
                &format!(
                    "position: fixed; \
                    pointer-events: none; \
                    z-index: 99999; \
                    font-size: 3px; \
                    line-height: 1; \
                    white-space: pre; \
                    color: #fff; \
                    text-shadow: 0 0 2px rgba(255,255,255,0.8); \
                    filter: drop-shadow(0 0 1px rgba(255,255,255,0.5)); \
                    left: {}px; \
                    top: {}px;",
                    self.neko_pos_x - 5.0, // Smaller offset for smaller cat
                    self.neko_pos_y - 5.0
                ),
            )?;
            Ok(())
        }
    }

    // ── ASCII scrollbar ────────────────────────────────────────────────────
    // A draggable, box-glyph scrollbar rendered under a `.scroll-container`
    // filmstrip. Reads the container's scroll state and draws `◄──███──►`; the
    // content is duplicated once (by scroll-container.js) for a seamless loop,
    // so the logical scroll range is scrollWidth/2.

    const BAR_CELLS: usize = 48;

    fn render_bar(container: &Element, bar: &HtmlPreElement) {
        let sw = container.scroll_width() as f64;
        let cw = container.client_width() as f64;
        let sl = container.scroll_left() as f64;
        let loop_w = sw / 2.0; // content duplicated once
        if loop_w <= cw || loop_w <= 0.0 {
            bar.set_text_content(Some(""));
            return;
        }
        let inner = BAR_CELLS - 2;
        let visible_frac = (cw / loop_w).clamp(0.0, 1.0);
        let thumb_len = ((visible_frac * inner as f64).round() as usize).clamp(1, inner);
        let pos = sl.rem_euclid(loop_w) / loop_w;
        let thumb_start = ((pos * inner as f64).round() as usize) % inner;
        let mut s = String::with_capacity(BAR_CELLS);
        s.push('◄');
        for i in 0..inner {
            let rel = (i + inner - thumb_start) % inner;
            s.push(if rel < thumb_len { '█' } else { '─' });
        }
        s.push('►');
        bar.set_text_content(Some(&s));
    }

    fn scroll_to_pointer(container: &Element, bar: &Element, client_x: f64) {
        let rect = bar.get_bounding_client_rect();
        if rect.width() <= 0.0 {
            return;
        }
        let frac = ((client_x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let sw = container.scroll_width() as f64;
        let cw = container.client_width() as f64;
        let loop_w = sw / 2.0;
        let target = (frac * loop_w).min(loop_w - cw).max(0.0);
        container.set_scroll_left(target as i32);
    }

    #[wasm_bindgen]
    pub struct AsciiScrollbar {
        // Closures are kept alive for the lifetime of the scrollbar (dropping the
        // struct detaches them). The struct is held by JS in an array.
        _closures: Vec<Closure<dyn FnMut(MouseEvent)>>,
        _render: Closure<dyn FnMut()>,
    }

    #[wasm_bindgen]
    impl AsciiScrollbar {
        #[wasm_bindgen(constructor)]
        pub fn new(container: HtmlElement) -> Result<AsciiScrollbar, JsValue> {
            let document = window()
                .ok_or("no window")?
                .document()
                .ok_or("no document")?;
            let win = window().ok_or("no window")?;

            let bar = document
                .create_element("pre")?
                .dyn_into::<HtmlPreElement>()?;
            bar.set_class_name("ascii-scrollbar");

            // Insert the bar right after the container.
            if let Some(parent) = container.parent_node() {
                parent.insert_before(&bar, container.next_sibling().as_ref())?;
            }
            // Flag the container so CSS hides the native scrollbar only when ours
            // is present (graceful fallback if the WASM never loads).
            let existing = container.class_name();
            container.set_class_name(&format!("{existing} has-ascii-scrollbar"));

            let dragging = Rc::new(Cell::new(false));
            let container_el: Element = container.clone().into();
            let bar_el: Element = bar.clone().into();

            // render() on scroll / resize / load.
            let render = {
                let c = container_el.clone();
                let b = bar.clone();
                Closure::wrap(Box::new(move || render_bar(&c, &b)) as Box<dyn FnMut()>)
            };
            let render_fn = render.as_ref().unchecked_ref();
            container_el.add_event_listener_with_callback("scroll", render_fn)?;
            win.add_event_listener_with_callback("resize", render_fn)?;
            win.add_event_listener_with_callback("load", render_fn)?;

            let mut closures: Vec<Closure<dyn FnMut(MouseEvent)>> = Vec::new();

            // mousedown on the bar → begin drag + jump to position.
            {
                let c = container_el.clone();
                let b = bar_el.clone();
                let d = dragging.clone();
                let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                    e.prevent_default();
                    d.set(true);
                    // Signal the JS auto-scroll to yield while we drag, so it
                    // doesn't overwrite scrollLeft every frame (the "snap back").
                    let _ = c.set_attribute("data-sb-drag", "1");
                    scroll_to_pointer(&c, &b, e.client_x() as f64);
                }) as Box<dyn FnMut(MouseEvent)>);
                bar.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }
            // mousemove on window → track while dragging.
            {
                let c = container_el.clone();
                let b = bar_el.clone();
                let d = dragging.clone();
                let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                    if d.get() {
                        scroll_to_pointer(&c, &b, e.client_x() as f64);
                    }
                }) as Box<dyn FnMut(MouseEvent)>);
                win.add_event_listener_with_callback("mousemove", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }
            // mouseup on window → end drag.
            {
                let d = dragging.clone();
                let c = container_el.clone();
                let cb = Closure::wrap(Box::new(move |_e: MouseEvent| {
                    d.set(false);
                    let _ = c.remove_attribute("data-sb-drag");
                }) as Box<dyn FnMut(MouseEvent)>);
                win.add_event_listener_with_callback("mouseup", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }

            render_bar(&container_el, &bar);

            Ok(AsciiScrollbar {
                _closures: closures,
                _render: render,
            })
        }
    }

    // ── ASCII lightbox ─────────────────────────────────────────────────────
    // Click a filmstrip image → open it in a draggable ASCII-framed window
    // (╔═ name ═ ✕ ╗). A press-and-drag on the strip scrolls it instead (a
    // small movement threshold tells a click from a drag). Reuses the existing
    // .ascii-frame CSS for the window border.

    const DRAG_THRESHOLD: f64 = 6.0; // px of movement before a press becomes a drag

    struct LbState {
        // Open window (+ its backdrop) and the listeners keeping it interactive.
        win: Option<HtmlElement>,
        backdrop: Option<HtmlElement>,
        win_closures: Vec<Closure<dyn FnMut(MouseEvent)>>,
        // Window drag.
        win_drag: bool,
        off_x: f64,
        off_y: f64,
        // Filmstrip press: grab-to-scroll + click detection.
        strip: Option<Element>,
        start_x: f64,
        start_scroll: f64,
        moved: bool,
        pending_src: String,
        pending_name: String,
    }

    #[wasm_bindgen]
    pub struct AsciiLightbox {
        _closures: Vec<Closure<dyn FnMut(MouseEvent)>>,
        _state: Rc<RefCell<LbState>>,
    }

    fn basename(src: &str) -> String {
        src.rsplit('/').next().unwrap_or(src).to_string()
    }

    fn close_window(state: &Rc<RefCell<LbState>>) {
        let mut s = state.borrow_mut();
        if let Some(win) = s.win.take() {
            if let Some(p) = win.parent_node() {
                let _ = p.remove_child(&win);
            }
        }
        if let Some(bd) = s.backdrop.take() {
            if let Some(p) = bd.parent_node() {
                let _ = p.remove_child(&bd);
            }
        }
        s.win_closures.clear();
        s.win_drag = false;
    }

    fn open_window(state: &Rc<RefCell<LbState>>, src: &str, name: &str) {
        close_window(state);
        let doc = match window().and_then(|w| w.document()) {
            Some(d) => d,
            None => return,
        };
        let body = match doc.body() {
            Some(b) => b,
            None => return,
        };

        // Backdrop (click to dismiss).
        let backdrop = match doc.create_element("div") {
            Ok(e) => e.dyn_into::<HtmlElement>().unwrap(),
            Err(_) => return,
        };
        backdrop.set_class_name("lb-backdrop");
        let _ = body.append_child(&backdrop);

        // Window.
        let win = match doc.create_element("div") {
            Ok(e) => e.dyn_into::<HtmlElement>().unwrap(),
            Err(_) => return,
        };
        win.set_class_name("ascii-lightbox");
        win.set_inner_html(&format!(
            "<div class=\"lb-titlebar\"><span class=\"lb-name\">{name}</span><span class=\"lb-close\">✕</span></div>\
             <img class=\"lb-img\" src=\"{src}\" alt=\"{name}\">\
             <div class=\"ascii-frame\" aria-hidden=\"true\">\
               <span class=\"ascii-frame-h ascii-frame-top\">╔<span class=\"ascii-frame-fill\"></span>╗</span>\
               <span class=\"ascii-frame-v ascii-frame-left\"></span>\
               <span class=\"ascii-frame-v ascii-frame-right\"></span>\
               <span class=\"ascii-frame-h ascii-frame-bottom\">╚<span class=\"ascii-frame-fill\"></span>╝</span>\
             </div>"
        ));
        let _ = body.append_child(&win);

        let mut win_closures: Vec<Closure<dyn FnMut(MouseEvent)>> = Vec::new();

        // Titlebar → start window drag (convert transform-centering to left/top).
        if let Ok(Some(titlebar)) = win.query_selector(".lb-titlebar") {
            let st = state.clone();
            let win_c = win.clone();
            let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                e.prevent_default();
                let rect = win_c.get_bounding_client_rect();
                let style = win_c.style();
                let _ = style.set_property("transform", "none");
                let _ = style.set_property("left", &format!("{}px", rect.left()));
                let _ = style.set_property("top", &format!("{}px", rect.top()));
                let mut s = st.borrow_mut();
                s.win_drag = true;
                s.off_x = e.client_x() as f64 - rect.left();
                s.off_y = e.client_y() as f64 - rect.top();
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ =
                titlebar.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
            win_closures.push(cb);
        }

        // ✕ → close.
        if let Ok(Some(close_btn)) = win.query_selector(".lb-close") {
            let st = state.clone();
            let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                e.stop_propagation();
                close_window(&st);
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = close_btn
                .add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
            win_closures.push(cb);
        }

        // Backdrop click → close.
        {
            let st = state.clone();
            let cb = Closure::wrap(
                Box::new(move |_e: MouseEvent| close_window(&st)) as Box<dyn FnMut(MouseEvent)>
            );
            let _ =
                backdrop.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
            win_closures.push(cb);
        }

        let mut s = state.borrow_mut();
        s.win = Some(win);
        s.backdrop = Some(backdrop);
        s.win_closures = win_closures;
    }

    #[wasm_bindgen]
    impl AsciiLightbox {
        #[wasm_bindgen(constructor)]
        pub fn new(container: HtmlElement) -> Result<AsciiLightbox, JsValue> {
            let win = window().ok_or("no window")?;
            let state = Rc::new(RefCell::new(LbState {
                win: None,
                backdrop: None,
                win_closures: Vec::new(),
                win_drag: false,
                off_x: 0.0,
                off_y: 0.0,
                strip: None,
                start_x: 0.0,
                start_scroll: 0.0,
                moved: false,
                pending_src: String::new(),
                pending_name: String::new(),
            }));

            let container_el: Element = container.clone().into();
            let mut closures: Vec<Closure<dyn FnMut(MouseEvent)>> = Vec::new();

            // Each image: press starts a strip grab; a release with no drag opens it.
            let imgs = container.query_selector_all("img")?;
            for i in 0..imgs.length() {
                let node = match imgs.get(i) {
                    Some(n) => n,
                    None => continue,
                };
                let img = match node.dyn_into::<Element>() {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let src = img.get_attribute("src").unwrap_or_default();
                let name = basename(&src);
                let st = state.clone();
                let cont = container_el.clone();
                let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                    e.prevent_default();
                    let mut s = st.borrow_mut();
                    s.strip = Some(cont.clone());
                    s.start_x = e.client_x() as f64;
                    s.start_scroll = cont.scroll_left() as f64;
                    s.moved = false;
                    s.pending_src = src.clone();
                    s.pending_name = name.clone();
                    // Pause the JS auto-scroll while pressed.
                    let _ = cont.set_attribute("data-sb-drag", "1");
                }) as Box<dyn FnMut(MouseEvent)>);
                img.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }

            // Global mousemove: drag the window, or grab-scroll the strip.
            {
                let st = state.clone();
                let cb = Closure::wrap(Box::new(move |e: MouseEvent| {
                    let mut s = st.borrow_mut();
                    if s.win_drag {
                        if let Some(win) = &s.win {
                            let style = win.style();
                            let _ = style.set_property(
                                "left",
                                &format!("{}px", e.client_x() as f64 - s.off_x),
                            );
                            let _ = style.set_property(
                                "top",
                                &format!("{}px", e.client_y() as f64 - s.off_y),
                            );
                        }
                    } else if let Some(cont) = s.strip.clone() {
                        let dx = e.client_x() as f64 - s.start_x;
                        if dx.abs() > DRAG_THRESHOLD {
                            s.moved = true;
                        }
                        cont.set_scroll_left((s.start_scroll - dx) as i32);
                    }
                }) as Box<dyn FnMut(MouseEvent)>);
                win.add_event_listener_with_callback("mousemove", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }

            // Global mouseup: end window drag; a strip press with no movement = a
            // click → open the pending image.
            {
                let st = state.clone();
                let cb = Closure::wrap(Box::new(move |_e: MouseEvent| {
                    let mut open: Option<(String, String)> = None;
                    {
                        let mut s = st.borrow_mut();
                        s.win_drag = false;
                        if let Some(cont) = s.strip.take() {
                            let _ = cont.remove_attribute("data-sb-drag");
                            if !s.moved {
                                open = Some((s.pending_src.clone(), s.pending_name.clone()));
                            }
                        }
                    }
                    if let Some((src, name)) = open {
                        open_window(&st, &src, &name);
                    }
                }) as Box<dyn FnMut(MouseEvent)>);
                win.add_event_listener_with_callback("mouseup", cb.as_ref().unchecked_ref())?;
                closures.push(cb);
            }

            Ok(AsciiLightbox {
                _closures: closures,
                _state: state,
            })
        }
    }
}

// Re-export for WASM target
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
