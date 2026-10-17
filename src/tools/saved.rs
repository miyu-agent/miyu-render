//! 存图：工作区里的 `rendered/`。一页先写进一个新建的临时文件再改名换上（预先放好的链接只会被换掉，不会被顺着写进去）；
//! 从最后一页往前写，第一页在就是整套都在。

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::render::Page;

/// 存图的文件夹，在工作区里。
pub const FOLDER: &str = "rendered";

/// 一次最多认几页画过的（画一次的页数有像素上限，到不了这么多）。
const MAX_PAGES: usize = 64;

/// 存好的一页。
#[derive(Debug, PartialEq, Eq)]
pub struct Saved {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// 第几页的文件名。
pub fn name(key: &str, page: usize) -> String {
    format!("render-{key}-{page}.png")
}

/// 工作区里的 `rendered/`：没有的建上；照真实的位置认，指到工作区外面的不用。
pub fn folder(workspace: &Path) -> Result<PathBuf, String> {
    let folder = workspace.join(FOLDER);
    fs::create_dir_all(&folder).map_err(|error| format!("Cannot create {FOLDER}/: {error}."))?;
    let real =
        fs::canonicalize(&folder).map_err(|error| format!("Cannot open {FOLDER}/: {error}."))?;
    if !real.starts_with(workspace) {
        return Err(format!("{FOLDER}/ points outside the workspace."));
    }
    Ok(real)
}

/// 画过的：第一页起连着的几页（都是普通文件、读得出长宽）；第一页没有的、有一页是链接的算没画过。
pub fn find(folder: &Path, key: &str) -> Option<Vec<Saved>> {
    let mut found = Vec::new();
    for page in 1..=MAX_PAGES {
        let name = name(key, page);
        let path = folder.join(&name);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            break;
        };
        if !meta.is_file() {
            return None;
        }
        let (width, height) = image::image_dimensions(&path).ok()?;
        found.push(Saved {
            name,
            width,
            height,
        });
    }
    (!found.is_empty()).then_some(found)
}

/// 存这几页，交回存好的（照页的先后）。
pub fn store(folder: &Path, key: &str, pages: &[Page]) -> Result<Vec<Saved>, String> {
    let mut saved = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate().rev() {
        let name = name(key, index + 1);
        replace(&folder.join(&name), &page.png)
            .map_err(|error| format!("Cannot save {FOLDER}/{name}: {error}."))?;
        saved.push(Saved {
            name,
            width: page.width,
            height: page.height,
        });
    }
    saved.reverse();
    Ok(saved)
}

/// 写进同一个文件夹里一个新建的临时文件（`create_new` 不跟链接），再改名换上 `path`。
fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_extension(format!(
        "png.{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| file.write_all(bytes))
        .and_then(|()| fs::rename(&temp, path));
    if written.is_err() {
        drop(fs::remove_file(&temp));
    }
    written
}

#[cfg(test)]
mod tests;
