//! 给别的软件调的方法（设计 36 第六节）：清单 `[export] methods = ["markdown"]`，核心发 `method.call {method, params, from?}`。
//!
//! `markdown {markdown, theme?}`：画成图，每一页经 `blob.put` 交核心，回 `{pages: [{blob, width, height}]}`。字最多
//! [`MAX_CHARS`] 个（同旧版）；`theme` 不写的照配置。字体第一次画的时候才挑，配置换了字体下一次重挑。

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use cosmic_text::fontdb::Database;
use serde_json::{Value, json};

use crate::fonts::Generic;
use crate::render::{Engine, Page, Style};
use crate::settings::Settings;
use crate::theme::Theme;
use crate::wire::{Answer, Wire};

/// 一次最多画多少个字（同旧版）。
pub const MAX_CHARS: usize = 20_000;

/// 登记的方法。
pub const NAMES: [&str; 1] = ["markdown"];

/// 核心那一头：测试里换成假的。
pub trait Core: Send + Sync {
    fn call(&self, method: &str, params: &Value) -> Answer;
}

impl Core for Wire {
    fn call(&self, method: &str, params: &Value) -> Answer {
        Wire::call(self, method, params)
    }
}

/// 字体库从哪来：正式跑是系统的，测试是仓库里的测试字体。
pub type Fonts = Arc<dyn Fn() -> (Database, Generic) + Send + Sync>;

/// 一次最多画多久（设计 36 第四节第 6 条）。
pub const TIME_LIMIT: Duration = Duration::from_secs(30);

/// 几样共用的：配置、画的那一套、核心、字体库从哪来、限时。
pub struct State {
    pub settings: Mutex<Settings>,
    /// 画的那一套：第一次画才开，字体换了、画崩了扔掉重开。一次只画一张。
    pub engine: Arc<Mutex<Option<Engine>>>,
    pub core: Arc<dyn Core>,
    pub fonts: Fonts,
    /// 一次最多画多久：正式跑是 [`TIME_LIMIT`]。
    pub limit: Duration,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `package.methods` 的参数。
pub fn register() -> Value {
    let methods: Vec<Value> = NAMES.iter().map(|name| json!({ "name": name })).collect();
    json!({ "methods": methods })
}

fn bad(why: &str) -> Value {
    json!({"code": -32602, "message": why})
}

fn failed(why: &str) -> Value {
    json!({"code": -32010, "message": why})
}

/// 办一次 `method.call`。
pub fn call(state: &State, request: &Value) -> Answer {
    match request["method"].as_str().unwrap_or_default() {
        "markdown" => markdown(state, &request["params"]),
        _ => Err(crate::wire::unknown_method()),
    }
}

fn markdown(state: &State, params: &Value) -> Answer {
    let text = params["markdown"]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| bad("markdown is required"))?;
    if text.chars().count() > MAX_CHARS {
        return Err(bad(&format!(
            "markdown is longer than {MAX_CHARS} characters"
        )));
    }
    let settings = locked(&state.settings).clone();
    let theme = match &params["theme"] {
        Value::Null => settings.theme,
        Value::String(name) => {
            Theme::parse(name).ok_or_else(|| bad("theme is paper, light or dark"))?
        }
        _ => return Err(bad("theme is paper, light or dark")),
    };
    let style = Style {
        palette: theme.palette(),
        font_size: settings.font_size,
    };
    let fonts = (settings.font.clone(), settings.code_font.clone());
    let pages = draw(state, text.to_string(), fonts, style)?;
    let mut answered = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let name = format!("render-{}.png", index + 1);
        let put = state
            .core
            .call(
                "blob.put",
                &json!({"data": STANDARD.encode(&page.png), "name": name}),
            )
            .map_err(|error| failed(&format!("blob.put failed: {error}")))?;
        let blob = put["blob"]
            .as_str()
            .ok_or_else(|| failed("blob.put gave no blob"))?;
        answered.push(json!({"blob": blob, "width": page.width, "height": page.height}));
    }
    Ok(json!({ "pages": answered }))
}

/// 在另一个线程里画，等至多 `state.limit`：画崩了（panic）接住、扔掉画的那一套（下一次重开）；超时的不等了，那一张画完自己扔掉。
fn draw(
    state: &State,
    text: String,
    (body, code): (Option<String>, Option<String>),
    style: Style,
) -> Result<Vec<Page>, Value> {
    let (engine, fonts) = (Arc::clone(&state.engine), Arc::clone(&state.fonts));
    let (tell, answer) = mpsc::channel();
    std::thread::spawn(move || {
        let mut held = locked(&engine);
        let done = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if held.is_none() {
                let (database, generic) = fonts();
                let opened = Engine::new(database, (body.as_deref(), code.as_deref()), &generic)?;
                if let Some(note) = &opened.families().note {
                    eprintln!("{note}");
                }
                *held = Some(opened);
            }
            Ok::<_, crate::fonts::FontError>(
                held.as_mut().map(|engine| engine.render(&text, &style)),
            )
        }));
        if done.is_err() {
            *held = None;
        }
        drop(tell.send(done));
    });
    match answer.recv_timeout(state.limit) {
        Ok(Ok(Ok(Some(Ok(pages))))) => Ok(pages),
        Ok(Ok(Ok(Some(Err(too_long))))) => Err(failed(&too_long.to_string())),
        Ok(Ok(Err(error))) => Err(failed(&error.to_string())),
        Ok(Ok(Ok(None))) => Err(failed("renderer not ready")),
        Ok(Err(_)) => Err(failed("the renderer crashed")),
        Err(_) => Err(failed("took too long to draw")),
    }
}

#[cfg(test)]
mod tests;
