//! 测试共用的：假核心、测试字体做的字体库、一份照它们拼的 [`State`]。

use std::path::Path;
use std::sync::{Arc, Mutex};

use cosmic_text::fontdb::Database;
use serde_json::{Value, json};

use crate::fonts::Generic;
use crate::methods::{Core, Fonts, State, TIME_LIMIT};
use crate::wire::Answer;

/// 假核心：记下 `blob.put` 收到的，回一个编号；`refuse` 的一律回错。
pub struct Fake {
    pub put: Mutex<Vec<Value>>,
    pub refuse: bool,
}

impl Core for Fake {
    fn call(&self, method: &str, params: &Value) -> Answer {
        assert_eq!(method, "blob.put");
        if self.refuse {
            return Err(json!({"code": -32010, "message": "disk full"}));
        }
        self.put.lock().expect("没 panic").push(params.clone());
        Ok(json!({"blob": "b1", "kind": "image", "media_type": "image/png"}))
    }
}

/// 测试字体做的字体库：`files` 是 `tests/fonts/` 里的。
pub fn fonts(files: &'static [&'static str]) -> Fonts {
    Arc::new(move || {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts");
        let mut database = Database::new();
        for file in files {
            database.load_font_file(dir.join(file)).expect("读得出");
        }
        (database, Generic::default())
    })
}

pub fn setup(refuse: bool, files: &'static [&'static str]) -> (State, Arc<Fake>) {
    let fake = Arc::new(Fake {
        put: Mutex::new(Vec::new()),
        refuse,
    });
    let state = State {
        settings: Mutex::new(crate::settings::Settings::default()),
        engine: Arc::new(Mutex::new(None)),
        core: Arc::clone(&fake) as Arc<dyn Core>,
        fonts: fonts(files),
        limit: TIME_LIMIT,
    };
    (state, fake)
}

/// 一个新的空文件夹（系统临时目录下），照真实的位置交回（macOS 的 `/var` 是链接）。
pub fn workspace() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "miyu-render-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    std::fs::canonicalize(&dir).expect("认得出")
}
