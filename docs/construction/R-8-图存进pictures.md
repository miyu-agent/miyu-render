## 施工单 R-8：画的图存进主人家目录的 pictures

状态：在做（本机门禁、5 个变异全抓到：去掉链接检查、不建目录、路径照工作区写、不认 `pictures`、空的不回退——最后一条起先活下来，补了 `an_empty_pictures_falls_back_to_the_workspace`）。

### 目的

核心 Y-11 下（设计 37 之后的「目录归属」）：会话里的 `tool.call` 多一格 `pictures`，是会话属主家目录里放图的目录（真实位置，可能还没建）。图存在那里，人在终端、网页里找得到，不再散落在各个工作区的 `rendered/` 里。没有这一格的（会话外面的调用、老的核心）照旧存工作区。

### 做什么

- `render_image`：有 `pictures` 的存进它（没有的连上层一起建），结果里的路径照它写；没有的照旧 `<cwd>/rendered/`。画过的不重画照旧按存的那个文件夹认。
- 那个目录本身是链接的不用（报错 `The pictures folder is a link.`）：核心给的是真实位置，变成链接只能是有人动过；上层的目录是主人自己的，不另查。页的文件照旧先写临时文件再改名，预先放好的链接被换掉。
- 读的 `.md` 照旧只认工作区里的。
- 工具说明第二句改成不提存在哪：`The images are saved as files and nobody sees them until you show them.`（存哪看结果里的路径）；登记簿重量 token。

### 测试（先红）

- `pages_go_to_the_pictures_folder_when_given`：给了 `pictures`（还没建、上层也没有），图存进去、结果是那里的路径、工作区里不建 `rendered/`。
- `a_linked_pictures_folder_is_refused`（unix）：`pictures` 是指到别处的链接，报错、别处没多东西。
- `an_empty_pictures_falls_back_to_the_workspace`：`pictures` 是空字的走工作区。
- 原有的工作区那几条照旧过（没有 `pictures` 的走老路）。

### 验收

本机门禁（格式、clippy、文档、测试、行数）；三台 CI。
