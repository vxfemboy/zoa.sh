// This file is only compiled for WASM target
#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::mods::shoutbox_protocol::{
        ShoutboxCommand as ProtoCommand, ShoutboxMessage as ProtoMessage,
    };
    use js_sys::Math;
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, Document, HtmlPreElement, MessageEvent, WebSocket};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    fn debug_enabled() -> bool {
        if let Some(win) = window() {
            if let Ok(val) = js_sys::Reflect::get(&win.into(), &JsValue::from_str("SADSITE_DEBUG"))
            {
                return val.as_bool().unwrap_or(false);
            }
        }
        false
    }

    macro_rules! console_log {
        ($($t:tt)*) => ({ if debug_enabled() { log(&format_args!($($t)*).to_string()) } })
    }

    const NEKO_SPEED: f64 = 10.0; // Movement speed per tick (~matches current animation cadence)

    // Local copies of box widths to avoid path resolution issues in wasm-only submodule
    const BOX_WIDTH_TINY: usize = 30;
    const BOX_WIDTH_SMALL: usize = 40;
    const BOX_WIDTH_MEDIUM: usize = 60;
    const BOX_WIDTH_LARGE: usize = 80;

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

        // Shoutbox client state
        ws: Option<WebSocket>,
        last_status: String,
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
                ws: None,
                last_status: String::new(),
            };

            // Set initial sprite
            cat.set_sprite("still")?;

            Ok(cat)
        }

        pub fn init_shoutbox(&mut self) -> Result<(), JsValue> {
            let location = window().ok_or("No window")?.location();
            let protocol = if location.protocol()?.as_str() == "https:" {
                "wss:"
            } else {
                "ws:"
            };
            let host = location.host()?;
            let url = format!("{}//{}/ws/shoutbox", protocol, host);

            let ws = WebSocket::new(&url)?;

            // Basic reconnect onclose
            let onclose = {
                let mut_self = self as *mut AsciiCat;
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| unsafe {
                    if let Some(inner) = mut_self.as_mut() {
                        inner.ws = None;
                        inner.last_status = "disconnected".into();
                    }
                })
            };
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();

            let onmessage_callback =
                Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                        let s: String = txt.into();
                        let _parsed: Result<ProtoMessage, _> = serde_json::from_str(&s);
                        // No DOM updates here; the page JS handles rendering
                    }
                });

            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();

            // Request initial messages
            let cmd = ProtoCommand::GetMessages;
            let json = serde_json::to_string(&cmd).unwrap_or_else(|_| "{}".to_string());
            let _ = ws.send_with_str(&json);

            self.ws = Some(ws);
            self.last_status = "connected".into();
            Ok(())
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

    // ────────────────────────────────────────────────────────────────────────
    // Shoutbox (WASM client)
    // ────────────────────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UiMessage {
        id: String,
        username: String,
        content: String,
        timestamp: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ActionEnvelope {
        action: String,
        messages: Option<Vec<UiMessage>>,
        message: Option<UiMessage>,
        user_count: Option<usize>,
        error: Option<String>,
    }

    struct ShoutboxClient {
        ws: Option<WebSocket>,
        tiny_id: String,
        small_id: String,
        medium_id: String,
        large_id: String,
        modal_id: String,
        is_expanded: bool,
        max_lines: usize,
        messages: Vec<UiMessage>,
        connected_users: usize,
    }

    thread_local! {
        static SHOUT: RefCell<Option<ShoutboxClient>> = RefCell::new(None);
    }

    fn get_document() -> Document {
        window().unwrap().document().unwrap()
    }

    fn set_pre_text(id: &str, text: &str) {
        if id.is_empty() {
            return;
        }
        let doc = get_document();
        if let Some(el) = doc.get_element_by_id(id) {
            if let Ok(pre) = el.dyn_into::<HtmlPreElement>() {
                pre.set_text_content(Some(text));
            }
        }
    }

    fn display_width(s: &str) -> usize {
        UnicodeWidthStr::width(s)
    }

    fn truncate_display(s: &str, max_cols: usize) -> String {
        if display_width(s) <= max_cols {
            return s.to_string();
        }
        let mut out = String::new();
        let mut cols = 0usize;
        for ch in s.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1);
            if cols + w > max_cols {
                break;
            }
            out.push(ch);
            cols += w;
        }
        out
    }

    fn pad_display(s: &str, total_cols: usize) -> String {
        let w = display_width(s);
        if w >= total_cols {
            truncate_display(s, total_cols)
        } else {
            let mut out = String::from(s);
            out.push_str(&" ".repeat(total_cols - w));
            out
        }
    }

    fn build_box(
        title: &str,
        icon: &str,
        width: usize,
        status: &str,
        lines: &[String],
        visible_lines: usize,
    ) -> String {
        let safe_width = width.max(20);
        let border_top = format!("╔{}╗", "═".repeat(safe_width));
        let border_bottom = format!("╚{}╝", "═".repeat(safe_width));
        let border_line = format!("╠{}╣", "═".repeat(safe_width));
        let empty_line = format!("║{}║", " ".repeat(safe_width));

        // Title with right-side icon
        let reserved = display_width(icon) + 1; // space + icon
        let title_w = display_width(title);
        let left_pad = ((safe_width.saturating_sub(title_w + reserved)) / 2).max(0);
        let right_pad = safe_width.saturating_sub(title_w + reserved + left_pad);
        let mut title_buf = String::new();
        title_buf.push_str(&" ".repeat(left_pad));
        title_buf.push_str(title);
        title_buf.push_str(&" ".repeat(right_pad));
        title_buf.push(' ');
        title_buf.push_str(icon);
        let title_line = format!("║{}║", pad_display(&title_buf, safe_width));

        let status_line = format!("║ {}  ║", pad_display(status, safe_width.saturating_sub(3)));

        let mut out = String::new();
        out.push_str(&border_top);
        out.push('\n');
        out.push_str(&title_line);
        out.push('\n');
        out.push_str(&border_line);
        out.push('\n');
        out.push_str(&empty_line);
        out.push('\n');
        out.push_str(&status_line);
        out.push('\n');
        out.push_str(&empty_line);
        out.push('\n');

        // Show the most recent content: take from the end of the buffer
        let total = lines.len();
        let start = total.saturating_sub(visible_lines);
        for i in start..total {
            let line = &lines[i];
            out.push_str(&format!(
                "║ {}  ║\n",
                pad_display(line, safe_width.saturating_sub(3))
            ));
        }
        // If there were fewer lines than the target height, pad the top with empties
        let remaining = visible_lines.saturating_sub(total);
        for _ in 0..remaining {
            out.push_str(&format!("{}\n", empty_line));
        }

        out.push_str(&empty_line);
        out.push('\n');
        out.push_str(&border_bottom);
        out
    }

    fn render() {
        SHOUT.with(|cell| {
            let mut_opt = cell.borrow();
            if let Some(client) = &*mut_opt {
                let is_expanded = client.is_expanded;
                // Match site box widths: posts use BOX_WIDTH_* as TOTAL width.
                // Our builder expects CONTENT width, so subtract 2 for borders.
                let max_widths = [
                    BOX_WIDTH_TINY.saturating_sub(2),
                    BOX_WIDTH_SMALL.saturating_sub(2),
                    BOX_WIDTH_MEDIUM.saturating_sub(2),
                    BOX_WIDTH_LARGE.saturating_sub(2),
                ];
                let status = format!(
                    "{} - {} users",
                    if client.ws.is_some() {
                        "Connected"
                    } else {
                        "Disconnected"
                    },
                    client.connected_users
                );

                // Build lines from messages
                let max_width = |vw: usize| vw.saturating_sub(2);
                let wrap = |text: &str, width: usize| -> Vec<String> {
                    let mut out = Vec::new();
                    let mut cur = String::new();
                    let mut cur_w = 0usize;
                    for word in text.split(' ') {
                        let ww = display_width(word);
                        if cur.is_empty() {
                            cur.push_str(word);
                            cur_w = ww;
                            continue;
                        }
                        if cur_w + 1 + ww <= width {
                            cur.push(' ');
                            cur.push_str(word);
                            cur_w += 1 + ww;
                        } else {
                            if !cur.is_empty() {
                                out.push(cur.clone());
                                cur.clear();
                                cur_w = 0;
                            }
                            if ww > width {
                                // Break the long word into display-width chunks
                                let mut buf = String::new();
                                let mut bw = 0usize;
                                for ch in word.chars() {
                                    let w = UnicodeWidthChar::width(ch).unwrap_or(1);
                                    if bw + w > width {
                                        out.push(buf.clone());
                                        buf.clear();
                                        bw = 0;
                                    }
                                    buf.push(ch);
                                    bw += w;
                                }
                                if !buf.is_empty() {
                                    cur = buf;
                                    cur_w = bw;
                                }
                            } else {
                                cur.push_str(word);
                                cur_w = ww;
                            }
                        }
                    }
                    if !cur.is_empty() {
                        out.push(cur);
                    }
                    out
                };

                let mut rendered_lines: Vec<String> = Vec::new();
                for m in &client.messages {
                    let uname = truncate_display(&m.username, 10);
                    let txt = format!("@{}: \"{}\"", uname, m.content);
                    let wrapped = wrap(&txt, max_width(BOX_WIDTH_LARGE.saturating_sub(2))); // wrap to largest width, smaller will pad/truncate
                    rendered_lines.extend(wrapped);
                }
                let lines = if rendered_lines.is_empty() {
                    vec!["No messages yet. Be the first to shout!".to_string()]
                } else {
                    rendered_lines
                };

                let icon = if is_expanded { "[x]" } else { "[+]" };
                // Choose visible lines: match site feel (collapsed near WHOAMI height)
                let vw_now = window()
                    .and_then(|w| w.inner_width().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(800.0);
                let visible = if is_expanded {
                    client.max_lines
                } else {
                    if vw_now <= 600.0 {
                        7
                    } else {
                        8
                    }
                };
                let single = client.small_id.is_empty()
                    && client.medium_id.is_empty()
                    && client.large_id.is_empty();
                if single {
                    // Use breakpoint-based widths to match site design precisely
                    let vw = window()
                        .and_then(|w| w.inner_width().ok())
                        .and_then(|v| v.as_f64())
                        .unwrap_or(800.0);
                    // Match main content boxes: use BOX_WIDTH_* minus borders by breakpoint
                    let width: usize = if vw <= 600.0 {
                        BOX_WIDTH_SMALL.saturating_sub(2) // Mobile uses small (40-char) boxes
                    } else if vw <= 900.0 {
                        BOX_WIDTH_MEDIUM.saturating_sub(2)
                    } else {
                        BOX_WIDTH_LARGE.saturating_sub(2)
                    };
                    let text = build_box("SHOUTBOX", icon, width, &status, &lines, visible);
                    set_pre_text(&client.tiny_id, &text);
                } else {
                    let ids = [
                        &client.tiny_id,
                        &client.small_id,
                        &client.medium_id,
                        &client.large_id,
                    ];
                    for (i, id) in ids.iter().enumerate() {
                        if !id.is_empty() {
                            let text = build_box(
                                "SHOUTBOX",
                                icon,
                                max_widths[i],
                                &status,
                                &lines,
                                visible,
                            );
                            set_pre_text(id, &text);
                        }
                    }
                }

                // Modal uses same box widths as collapsed shoutbox
                let doc = get_document();
                if let Some(el) = doc.get_element_by_id(&client.modal_id) {
                    if let Ok(pre) = el.dyn_into::<HtmlPreElement>() {
                        let vw = window()
                            .and_then(|w| w.inner_width().ok())
                            .and_then(|v| v.as_f64())
                            .unwrap_or(800.0);
                        // Use same box widths as the collapsed boxes
                        let modal_width: usize = if vw <= 600.0 {
                            BOX_WIDTH_SMALL.saturating_sub(2)
                        } else if vw <= 900.0 {
                            BOX_WIDTH_MEDIUM.saturating_sub(2)
                        } else {
                            BOX_WIDTH_LARGE.saturating_sub(2)
                        };
                        let text =
                            build_box("SHOUTBOX", icon, modal_width, &status, &lines, visible);
                        pre.set_text_content(Some(&text));
                    }
                }
            }
        });
    }

    fn connect_ws() {
        SHOUT.with(|cell| {
            let mut s = cell.borrow_mut();
            let client = match s.as_mut() {
                Some(c) => c,
                None => return,
            };
            // Don't reconnect if already connected
            if client.ws.is_some() {
                return;
            }
            let win = match window() {
                Some(w) => w,
                None => {
                    if debug_enabled() {
                        web_sys::console::log_1(&"No window".into());
                    }
                    return;
                }
            };
            let location = win.location();
            let raw_protocol = location.protocol().unwrap_or_else(|_| "http:".into());
            let protocol = if raw_protocol == "https:" {
                "wss:"
            } else {
                "ws:"
            };
            let host = location.host().unwrap_or_else(|_| {
                if debug_enabled() {
                    web_sys::console::log_1(
                        &"location.host failed; defaulting to localhost:8080".into(),
                    );
                }
                String::from("localhost:8080")
            });
            let url = format!("{}//{}/ws/shoutbox", protocol, host);
            if debug_enabled() {
                web_sys::console::log_1(&format!("Connecting WebSocket to {}", url).into());
            }
            let ws = match WebSocket::new(&url) {
                Ok(w) => w,
                Err(e) => {
                    if debug_enabled() {
                        web_sys::console::log_1(
                            &format!("WebSocket connect failed: {:?}", e).into(),
                        );
                    }
                    return;
                }
            };

            // Store WebSocket so we can send later (e.g., shoutbox_send)
            client.ws = Some(ws.clone());

            // onmessage via set_onmessage using generic JsValue for maximum compatibility
            let onmessage = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
                if let Ok(ev) = e.clone().dyn_into::<web_sys::MessageEvent>() {
                    if let Some(s) = ev.data().as_string() {
                        if debug_enabled() {
                            web_sys::console::log_1(&format!("WS message: {}", s).into());
                        }
                        let mut should_render = false;
                        SHOUT.with(|cell| {
                            let mut guard = cell.borrow_mut();
                            if let Some(client) = guard.as_mut() {
                                if let Ok(env) = serde_json::from_str::<ActionEnvelope>(&s) {
                                    match env.action.as_str() {
                                        "messages" => {
                                            if let Some(list) = env.messages {
                                                client.messages = list;
                                            }
                                        }
                                        "new_message" => {
                                            if let Some(msg) = env.message {
                                                client.messages.push(msg);
                                                if client.messages.len() > 50 {
                                                    client.messages.remove(0);
                                                }
                                            }
                                        }
                                        "user_count" => {
                                            if let Some(c) = env.user_count {
                                                client.connected_users = c;
                                            }
                                        }
                                        _ => {}
                                    }
                                    should_render = true;
                                } else if let Ok(proto) = serde_json::from_str::<ProtoMessage>(&s) {
                                    match proto {
                                        ProtoMessage::MessageList(list) => {
                                            client.messages = list
                                                .into_iter()
                                                .map(|m| UiMessage {
                                                    id: m.id,
                                                    username: m.username,
                                                    content: m.content,
                                                    timestamp: m.timestamp,
                                                })
                                                .collect();
                                        }
                                        ProtoMessage::NewMessage {
                                            id,
                                            username,
                                            content,
                                            timestamp,
                                        } => {
                                            client.messages.push(UiMessage {
                                                id,
                                                username,
                                                content,
                                                timestamp,
                                            });
                                        }
                                        ProtoMessage::UserCount(n) => {
                                            client.connected_users = n;
                                        }
                                        _ => {}
                                    }
                                    should_render = true;
                                }
                            }
                        });
                        if should_render {
                            render();
                        }
                    }
                } else if let Some(s) = e.as_string() {
                    if debug_enabled() {
                        web_sys::console::log_1(&format!("WS message: {}", s).into());
                    }
                    let mut should_render = false;
                    SHOUT.with(|cell| {
                        let mut guard = cell.borrow_mut();
                        if let Some(client) = guard.as_mut() {
                            if let Ok(env) = serde_json::from_str::<ActionEnvelope>(&s) {
                                match env.action.as_str() {
                                    "messages" => {
                                        if let Some(list) = env.messages {
                                            client.messages = list;
                                        }
                                    }
                                    "new_message" => {
                                        if let Some(msg) = env.message {
                                            client.messages.push(msg);
                                            if client.messages.len() > 50 {
                                                client.messages.remove(0);
                                            }
                                        }
                                    }
                                    "user_count" => {
                                        if let Some(c) = env.user_count {
                                            client.connected_users = c;
                                        }
                                    }
                                    _ => {}
                                }
                                should_render = true;
                            } else if let Ok(proto) = serde_json::from_str::<ProtoMessage>(&s) {
                                match proto {
                                    ProtoMessage::MessageList(list) => {
                                        client.messages = list
                                            .into_iter()
                                            .map(|m| UiMessage {
                                                id: m.id,
                                                username: m.username,
                                                content: m.content,
                                                timestamp: m.timestamp,
                                            })
                                            .collect();
                                    }
                                    ProtoMessage::NewMessage {
                                        id,
                                        username,
                                        content,
                                        timestamp,
                                    } => {
                                        client.messages.push(UiMessage {
                                            id,
                                            username,
                                            content,
                                            timestamp,
                                        });
                                    }
                                    ProtoMessage::UserCount(n) => {
                                        client.connected_users = n;
                                    }
                                    _ => {}
                                }
                                should_render = true;
                            }
                        }
                    });
                    if should_render {
                        render();
                    }
                }
            });
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            // onopen: request initial messages using legacy envelope
            let ws_open = ws.clone();
            let onopen = Closure::<dyn FnMut(JsValue)>::new(move |_e: JsValue| {
                if debug_enabled() {
                    web_sys::console::log_1(&"WS open".into());
                }
                let init = serde_json::json!({ "action": "get_messages" }).to_string();
                let _ = ws_open.send_with_str(&init);
                // Also request user_count via legacy envelope to prompt a server push if implemented
                let _ =
                    ws_open.send_with_str(&serde_json::json!({"action":"user_count"}).to_string());
                // Force a render to update status line immediately
                render();
            });
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            // onerror: log
            let onerror = Closure::<dyn FnMut(JsValue)>::new(move |_e: JsValue| {
                if debug_enabled() {
                    web_sys::console::log_1(&"WebSocket error".into());
                }
            });
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            // onclose: clear socket and attempt reconnect after short delay
            let onclose = Closure::<dyn FnMut(JsValue)>::new(move |_e: JsValue| {
                if debug_enabled() {
                    web_sys::console::log_1(&"WebSocket closed; scheduling reconnect".into());
                }
                // Clear socket state
                SHOUT.with(|cell| {
                    if let Some(client) = cell.borrow_mut().as_mut() {
                        client.ws = None;
                    }
                });
                // Schedule reconnect in ~1.5s
                if let Some(win) = window() {
                    let cb = Closure::<dyn FnMut()>::once(move || {
                        connect_ws();
                        render();
                    });
                    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        1500,
                    );
                    cb.forget();
                }
                render();
            });
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();
        });
    }

    #[wasm_bindgen]
    pub fn shoutbox_reconnect() {
        connect_ws();
    }

    #[wasm_bindgen]
    pub fn init_shoutbox_client(
        tiny_id: &str,
        small_id: &str,
        medium_id: &str,
        large_id: &str,
        modal_id: &str,
    ) {
        SHOUT.with(|cell| {
            *cell.borrow_mut() = Some(ShoutboxClient {
                ws: None,
                tiny_id: tiny_id.to_string(),
                small_id: small_id.to_string(),
                medium_id: medium_id.to_string(),
                large_id: large_id.to_string(),
                modal_id: modal_id.to_string(),
                is_expanded: false,
                max_lines: 5,
                messages: Vec::new(),
                connected_users: 0,
            });
        });
        connect_ws();
        render();
    }

    #[wasm_bindgen]
    pub fn init_shoutbox_client_single(collapsed_id: &str, modal_id: &str) {
        SHOUT.with(|cell| {
            *cell.borrow_mut() = Some(ShoutboxClient {
                ws: None,
                tiny_id: collapsed_id.to_string(),
                small_id: String::new(),
                medium_id: String::new(),
                large_id: String::new(),
                modal_id: modal_id.to_string(),
                is_expanded: false,
                max_lines: 5,
                messages: Vec::new(),
                connected_users: 0,
            });
        });
        connect_ws();
        render();
    }

    #[wasm_bindgen]
    pub fn shoutbox_set_expanded(expanded: bool, lines: usize) {
        SHOUT.with(|cell| {
            if let Some(client) = cell.borrow_mut().as_mut() {
                client.is_expanded = expanded;
                client.max_lines = lines.max(1);
            }
        });
        render();
    }

    #[wasm_bindgen]
    pub fn shoutbox_send(username: String, content: String) {
        let ws_opt = SHOUT.with(|cell| cell.borrow().as_ref().and_then(|c| c.ws.clone()));
        if let Some(ws) = ws_opt {
            let cmd = serde_json::json!({
                "action": "send_message",
                "username": username,
                "content": content
            });
            let _ = ws.send_with_str(&cmd.to_string());
        }
    }
}

// Re-export for WASM target
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
