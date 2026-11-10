//! 测试共用的：假核心、测试字体做的字体库、一份照它们拼的 [`State`]。

use std::path::Path;
use std::sync::{Arc, Mutex};

use cosmic_text::fontdb::Database;
use serde_json::{Value, json};

use crate::fonts::Generic;
use crate::methods::{Core, Fonts, State, TIME_LIMIT};
use crate::wire::Answer;

/// 假核心：记下交来的每一张（`blob.put` 的原样；分块上传的拼好、记成 `blob.put` 的样子），回一个编号；`refuse` 的一律回错。
/// 照真核心的规矩查：一行不超过 1 MiB、一块不超过 512 KiB、`offset` 接得上、关的时候字节数对得上。
pub struct Fake {
    pub put: Mutex<Vec<Value>>,
    pub refuse: bool,
    /// 在传的那一张：名字、说好的字节数、收到的。
    pub upload: Mutex<Option<(String, usize, Vec<u8>)>>,
    /// 收到过的方法，照先后。
    pub methods: Mutex<Vec<String>>,
}

impl Core for Fake {
    fn call(&self, method: &str, params: &Value) -> Answer {
        use base64::Engine as _;
        use base64::engine::general_purpose::STANDARD;
        if self.refuse {
            return Err(json!({"code": -32010, "message": "disk full"}));
        }
        assert!(
            params.to_string().len() < 1024 * 1024 - 256,
            "{method} 的一行超过 1 MiB，真核心拒 line_too_long"
        );
        self.methods
            .lock()
            .expect("没 panic")
            .push(method.to_string());
        let mut upload = self.upload.lock().expect("没 panic");
        match method {
            "blob.put" => self.put.lock().expect("没 panic").push(params.clone()),
            "blob.open" => {
                let name = params["name"].as_str().expect("有名字").to_string();
                let size =
                    usize::try_from(params["size"].as_u64().expect("有大小")).expect("放得下");
                *upload = Some((name, size, Vec::new()));
                return Ok(json!({"upload": "u1"}));
            }
            "blob.write" => {
                let (_, _, got) = upload.as_mut().expect("先 open");
                assert_eq!(params["upload"], "u1");
                assert_eq!(params["offset"], got.len(), "offset 接得上");
                let chunk = STANDARD
                    .decode(params["data"].as_str().expect("有内容"))
                    .expect("是 base64");
                assert!(chunk.len() <= 512 * 1024, "一块最多 512 KiB");
                got.extend(chunk);
                return Ok(json!({"received": got.len()}));
            }
            "blob.close" => {
                let (name, size, got) = upload.take().expect("先 open");
                assert_eq!(got.len(), size, "字节数对得上");
                self.put
                    .lock()
                    .expect("没 panic")
                    .push(json!({"data": STANDARD.encode(got), "name": name}));
            }
            other => panic!("没想到的方法 {other}"),
        }
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
        upload: Mutex::new(None),
        methods: Mutex::new(Vec::new()),
    });
    let state = State {
        settings: Mutex::new(crate::settings::Settings::default()),
        engine: Arc::new(crate::methods::Pool::default()),
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
