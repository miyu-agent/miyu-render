<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

# Miyu 渲染（miyu-render）

> <sub>本文档由 Anthropic 的 Claude Fable 5.2 生成。</sub>

这个仓库是 Miyu 的渲染软件包（编号 `render`）。它把 Markdown 渲染成 PNG 图片，代码高亮、表格、公式、Mermaid 图表都能渲染。它主要供通讯平台使用：回复太长时，通讯平台会把回复渲染成图片发出去；在终端界面、网页界面里，你也可以让 AI 渲染图片。

## 安装

先安装 Miyu，然后运行：

```sh
miyu pkg install render
```

安装以后软件包会自动运行，不需要手动打开，也不需要下载模型。这个软件包可以安装在 `linux-x86_64`、`macos-aarch64`、`windows-x86_64` 上。安装以后会多出以下内容：

- AI 使用的工具 `render_image`：把一段 Markdown 或工作区里的 `.md` 文件渲染成图片，保存成文件，并返回文件路径。
- 总线方法 `render.markdown`：其他软件包传入 Markdown，得到一页一页的图片。通讯平台把长回复转成图片时使用的就是这个方法。
- 设置键 `render.theme`、`render.font`、`render.code_font`、`render.font_size`。

能渲染哪些内容、图片存在哪里、每项设置的含义、出了问题怎么处理，可以看 [附加软件：渲染](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/addons.md#渲染-render)。

渲染使用系统里已经安装的字体，不会下载字体。如果系统里没有任何一个支持中文的字体，就无法渲染：在 Linux 上需要安装 Noto Sans CJK（例如 `noto-fonts-cjk` 软件包）。

## 开发

这个软件包是一个 Rust 程序 `miyu-render`，只通过协议和核心通信，不依赖核心的任何 Rust 库（crate）。软件包文件夹是 `package/`，里面只有清单 `package.toml`。

| 目录 | 内容 |
|---|---|
| `src/markdown/` | 解析 Markdown（使用 `pulldown-cmark`） |
| `src/render/` | 排版、绘图（使用 `cosmic-text`、`resvg`；公式使用 `ratex`，KaTeX 的字体编译进程序；Mermaid 使用核心仓库里 `third_party/mermaid-rs-renderer` 的那一份，由 `Cargo.toml` 固定到一个提交） |
| `src/fonts/` | 选择系统字体：在 Linux 上查询 `fontconfig`，在 macOS、Windows 上使用默认的字体名 |
| `src/tools/`、`src/methods/` | 工具 `render_image`、总线方法 `render.markdown` |
| `tests/protocol.rs` | 启动 `miyu-render serve`，像核心那样通过协议和它通信 |
| `tests/fonts/` | 测试用的字体，经过裁剪、改了名；字体的来源和许可证见 `tests/fonts/README.md` |
| `docs/prompt-ledger.md` | 给模型看的文字（工具说明、参数说明、工具结果）的登记簿 |

测试不依赖运行测试的电脑上安装了哪些字体，只使用 `tests/fonts/` 里的字体。

开发需要 Rust，版本以 `rust-toolchain.toml` 为准，`rustup` 会自动安装这个版本。CI（`.github/workflows/ci.yml`）在 Linux、macOS、Windows 上运行以下检查：

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked
cargo test --locked --no-fail-fast
```

除此之外，还要检查每个 `.rs` 文件不超过 500 行，测试文件也算在内。

在本机试装的方法是：把编译好的程序放进软件包文件夹的 `bin/`，再把软件包文件夹拷贝成一个名叫 `render` 的文件夹并安装（软件包的编号由文件夹名决定）：

```sh
cargo build --release --locked
mkdir -p package/bin
cp target/release/miyu-render package/bin/
cp -R package /tmp/render
miyu pkg install /tmp/render
```

在 Windows 上，程序的文件名带 `.exe`。发布新版本时推送 `v<版本>` 的标签，`.github/workflows/release.yml` 会为每种系统打包一个 `.miyu` 文件，上传到发行版。标签的版本以 `package/package.toml` 里的 `version` 为准（`Cargo.toml` 里的版本要跟着一起修改）。

## 许可证

这个软件包的许可证是 AGPL-3.0-or-later，详细的见 `LICENSE`。依赖使用各自的 `license` 里写明的许可证；测试字体使用 `tests/fonts/` 里随附的许可证。
