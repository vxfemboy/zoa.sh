// This file is only compiled for WASM target
#[cfg(target_arch = "wasm32")]
mod wasm {
    use js_sys::Math;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, HtmlPreElement};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    macro_rules! console_log {
        ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
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
}

// Re-export for WASM target
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
