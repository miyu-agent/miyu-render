## 施工单 R-9：清单里配色的几样写成 choices

状态：做完了，等主会话审。

### 目的

2026-10-27 给项目主人本机装渲染包时，核心拒了清单：`[settings.theme]` 写的是 `options = [...]`，核心的清单只认 `choices`（`miyu_config::package`，一直是这样）。装不上，长文转图就没生效。部署那一份当场改了，仓库里的照旧错着。

测试没拦住：这个包不依赖核心的 crate（`AGENTS.md` 第 1 条），没有一道照核心的读法查清单。

### 做什么

- `package/package.toml`：`[settings.theme]` 的 `options` 改成 `choices`。
- `settings/tests.rs` 加一条：照行读清单，`[settings.theme]` 没有 `options`、有 `choices`，三套配色的名字都在。不引核心的 crate，只守这一处写法。

### 测试（先红）

- `the_theme_setting_lists_its_choices_the_way_the_core_reads_them`：把清单改回 `options`，这条转红。

### 验收

本机门禁（格式、clippy、文档、测试、行数）；三台 CI；真核心 `pkg install` 装得上（本机部署那一份已经照这样装上，长文转图出了图）。
