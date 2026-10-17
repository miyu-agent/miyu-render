//! miyu-render：Miyu 的渲染软件包（核心仓库设计 36）。核心照清单 `[process] args = ["serve"]` 拉起，标准输入输出上说协议。

mod fonts;
mod markdown;
mod methods;
mod render;
mod settings;
mod theme;
mod wire;

use std::io::{self, BufRead};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

use cosmic_text::fontdb::Database;
use serde_json::{Value, json};

use methods::State;
use settings::Settings;
use wire::Wire;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("serve") {
        return serve();
    }
    eprintln!("usage: miyu-render serve");
    ExitCode::from(2)
}

/// 系统的字体库和通用名：第一次画的时候才读。
fn system_fonts() -> (Database, fonts::Generic) {
    let mut database = Database::new();
    database.load_system_fonts();
    (database, fonts::system::generic())
}

/// 跑起来：握手、登记方法，然后照核心发来的办，标准输入读到头就退出。
fn serve() -> ExitCode {
    let wire = Arc::new(Wire::new(Box::new(io::stdout())));
    let state = Arc::new(State {
        settings: Mutex::new(Settings::default()),
        engine: Mutex::new(None),
        core: Arc::clone(&wire) as Arc<dyn methods::Core>,
        fonts: Box::new(system_fonts),
    });
    let reading = {
        let (wire, state) = (Arc::clone(&wire), Arc::clone(&state));
        std::thread::spawn(move || read(&wire, &state))
    };
    let hello = json!({"protocol": [1, 1], "head": {"kind": "miyu-render", "version": env!("CARGO_PKG_VERSION")}});
    match wire.call("hello", &hello) {
        Ok(answer) => {
            if let Some(config) = answer["config"].as_object() {
                configure(&state, config);
            }
        }
        Err(error) => {
            eprintln!("hello refused: {error}");
            return ExitCode::from(2);
        }
    }
    if let Err(error) = wire.call("package.methods", &methods::register()) {
        eprintln!("package.methods refused: {error}");
        return ExitCode::from(2);
    }
    drop(reading.join());
    ExitCode::SUCCESS
}

/// 配置换了：字体换了的扔掉画的那一套，下一次重挑。
fn configure(state: &State, keys: &serde_json::Map<String, Value>) {
    let fonts_moved = state
        .settings
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .apply(keys);
    if fonts_moved {
        *state.engine.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

/// 读核心发来的：回应交给等它的，请求各开一个线程办，通知当场办。
fn read(wire: &Arc<Wire>, state: &Arc<State>) {
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match (
            message.get("method").and_then(Value::as_str),
            message.get("id"),
        ) {
            (Some(method), Some(id)) => {
                let (wire, state) = (Arc::clone(wire), Arc::clone(state));
                let (method, id, params) =
                    (method.to_string(), id.clone(), message["params"].clone());
                std::thread::spawn(move || {
                    let answer = match method.as_str() {
                        "method.call" => methods::call(&state, &params),
                        _ => Err(wire::unknown_method()),
                    };
                    wire.answer(&id, answer);
                });
            }
            (Some("extension.config"), None) => {
                if let Some(keys) = message["params"]["keys"].as_object() {
                    configure(state, keys);
                }
            }
            (None, Some(id)) => {
                let answer = match message.get("error") {
                    Some(error) => Err(error.clone()),
                    None => Ok(message["result"].clone()),
                };
                wire.deliver(id.as_str().unwrap_or_default(), answer);
            }
            (Some(_) | None, None) => {}
        }
    }
}
