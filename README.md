# QuickGif

QuickGif 是一款轻量级跨平台桌面小工具，用于从视频中快速截取片段并导出 MP4、WebP 或 GIF，也支持导出当前帧 PNG。当前实现支持选择或拖入视频、读取视频信息、生成真实缩略图时间轴、建立精确帧时间戳索引、逐帧设置起止范围，并用内置 FFmpeg 完成导出。

## 技术栈

- Tauri 2
- React + TypeScript + Vite
- Rust 后端
- FFmpeg / ffprobe 作为随包资源

## Android 原生版

仓库同时新增了原生 Android MVP 子项目，位于：

```text
android/
```

Android 版使用 Kotlin、Jetpack Compose、AndroidX Media3 / ExoPlayer，并通过内置 native FFmpeg executable 执行 GIF 导出。第一阶段先跑通“选择视频 → 手动设置起止时间 → `palettegen + paletteuse` 两步法导出 GIF → MediaStore 保存到 `Movies/QuickGif`”主链路。

Android 开发、运行、FFmpeg 集成和 APK 打包说明见：

```text
android/README.md
```

## 功能状态

已完成：

- 选择视频文件
- 拖拽视频文件到窗口
- 支持 `mp4`、`mov`、`avi`、`mkv`、`webm`
- 使用 ffprobe 读取时长、分辨率、帧率、编码、总帧数
- 自动生成最多 80 张缩略图时间轴
- 视频预览播放
- 基于 ffprobe 帧时间戳的精确定位；播放器控制支持上一帧 / 下一帧
- 当前帧号和当前时间显示
- 点击缩略图或时间轴定位视频
- 设置开始帧 / 结束帧
- 手动输入开始时间 / 结束时间
- 导出 MP4、WebP、GIF
- 导出苹果实况照片（Live Photo：配对 JPG + MOV + Content Identifier）
- 导入实况照片静态图时自动解析同名 `.MOV` / `.MP4` 配对视频
- 右键导出当前帧 PNG
- 将开始帧到结束帧逐帧导出为连续编号的 PNG 图片
- 菜单栏选择导出格式，默认 MP4
- 基于 FFmpeg `-progress pipe:1` 的导出进度条
- 文件不存在、格式不支持、FFmpeg 调用失败、导出失败等错误提示
- 导出完成后显示保存路径并可打开文件夹
- 快捷键：默认 Space、Left、Right、I、O、Ctrl/Cmd + E，可在菜单栏设置中修改
- macOS `.app` / `.dmg` 与 Windows NSIS 安装包配置

核心后端模块：

- `video_probe`：读取视频基础信息
- `thumbnail_generator`：生成缩略图时间轴
- `frame_indexer`：建立帧号与时间戳映射
- `mp4_exporter`：导出高质量 H.264 MP4
- `webp_exporter`：导出循环动画 WebP
- `gif_exporter`：两步法导出 GIF
- `live_photo_exporter`：导出苹果实况照片（JPG + MOV 配对）
- `apple_live_photo_meta`：向 JPEG 写入 Content Identifier
- `image_exporter`：导出当前帧 PNG
- `temp_manager`：管理缩略图、调色板等临时文件
- `error_handler`：统一错误信息

## FFmpeg / ffprobe

QuickGif 后端只使用 Rust `Command` 参数数组调用 FFmpeg，不拼接 shell 字符串，因此支持中文路径和带空格路径。

查找顺序：

1. `QUICKGIF_FFMPEG_PATH` / `QUICKGIF_FFPROBE_PATH`
2. `QUICKGIF_FFMPEG_DIR`
3. Tauri resources 中的 `binaries/ffmpeg`、`binaries/ffprobe`
4. 开发环境 `PATH` 中的 `ffmpeg`、`ffprobe`

项目使用 npm 静态包准备二进制。运行桌面开发或打包命令前会自动执行：

```bash
npm run prepare:ffmpeg
```

该脚本会把当前平台的二进制复制到：

```text
src-tauri/binaries/
```

macOS/Linux 文件名为 `ffmpeg`、`ffprobe`，Windows 文件名为 `ffmpeg.exe`、`ffprobe.exe`。该目录下的实际二进制默认不提交到 Git，但会在本机打包时作为 Tauri resources 进入安装包。

## 开发

安装依赖：

```bash
npm install
```

启动桌面开发环境：

```bash
npm run tauri:dev
```

仅构建前端：

```bash
npm run build
```

运行 Rust 测试：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## 打包

macOS：

```bash
npm run tauri:build
```

产物包含 `.app` / `.dmg`。

Windows：

```bash
npm run tauri:build
```

在 Windows 环境执行后，产物包含 NSIS 安装包。

## 导出策略

导出时默认保存到原视频同目录，也可以在菜单栏设置默认导出文件夹。如果已存在同名文件，则自动追加序号。

菜单栏支持中文和英文两套显示语言。通过 `语言` / `Language` 菜单切换后，原生菜单和应用界面会同步使用对应语言。

菜单栏的 `导出` / `Export` → `导出格式` / `Export Format` 可选择：

- MP4：默认格式，H.264，适合体积较小的动图替代物
- WebP：动画 WebP，默认 15fps、720 边界内、无限循环
- GIF：默认 720 宽、15fps、标准质量
- 实况照片 / Live Photo：导出同主文件名的 `.JPG` + `.MOV` 配对，写入共享 Content Identifier；选区时长需在 **0.5–6 秒**。关键静帧默认取播放头（在选区内时）或选区中点。将两个文件一并导入 macOS/iOS「照片」即可识别为实况照片。

导入时若选择实况照片的静态图（`jpg` / `jpeg` / `heic` / `heif` / `png`），应用会自动查找同目录下同名的 `.MOV` / `.MP4` 作为编辑源。

在细节缩略图或视频区右键可导出当前帧 PNG；也可使用“逐帧导出选区”，将选区内原始帧输出到独立文件夹。

## GIF 导出策略

FFmpeg 流程：

1. 按开始时间和结束时间截取片段
2. 根据宽度和帧率设置滤镜
3. 使用 `scale=...:flags=lanczos`
4. 生成 `palette.png`
5. 使用调色板导出无限循环 GIF

质量映射：

- 小体积：128 色，bayer 抖动
- 标准：192 色，sierra2_4a 抖动
- 高清：256 色，sierra2_4a 抖动
