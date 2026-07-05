# QuickGif Android

QuickGif Android 是原生 Kotlin + Jetpack Compose 版本。第一阶段 MVP 跑通：

1. 从系统相册或文件管理器选择视频。
2. 使用 Media3 / ExoPlayer 预览视频。
3. 读取视频基础信息。
4. 手动输入开始秒和结束秒。
5. 通过内置 FFmpeg 执行 `palettegen + paletteuse` 两步法导出 GIF。
6. 使用 MediaStore 保存到 `Movies/QuickGif`。
7. 导出完成后打开或分享 GIF。

第二阶段已补齐：

1. 选择视频后后台生成最多 80 张缩略图。
2. 横向缩略图时间轴支持滑动浏览。
3. 点击缩略图定位视频预览。
4. 时间轴显示当前帧、开始帧、结束帧标记。
5. 支持上一帧、下一帧。
6. 支持将当前帧设置为开始帧或结束帧，并同步导出起止时间。

当前逐帧定位基于视频元数据中的帧率和可读取总帧数建立 `frameIndex -> timestamp` 映射，并对 Media3 播放器使用 exact seek。少数可变帧率或元数据缺失的视频可能只能退化为时间戳近似定位。

## 模块

- `app`：`MainActivity`、Compose 入口、系统选择器入口。
- `ui`：`VideoPickerScreen`、`VideoPreview`、`ThumbnailTimeline`、`FrameControlPanel`、`ExportSettingPanel`、`ExportProgressDialog`。
- `media`：`VideoInfoReader`、`FrameTimestampReader`、`ThumbnailExtractor`、`VideoPlayerController`。
- `ffmpeg`：`FFmpegManager`、`FFmpegCommandBuilder`、`GifExporter`、`ProgressParser`、`NativeLibraryLoader`。
- `data`：`VideoInfo`、`FrameInfo`、`GifExportConfig`、`ExportState`。
- `storage`：`MediaStoreSaver`、`TempFileManager`、`OutputPathManager`。
- `error`：`AppError`、`ErrorMapper`、`UserFriendlyErrorMessage`。

## FFmpeg 集成

本项目不依赖 FFmpegKit。MVP 使用 Android 内置 native executable 方案：

```text
android/app/src/main/jniLibs/arm64-v8a/libffmpeg.so
```

要求这个文件是为 Android arm64-v8a 编译的 FFmpeg PIE executable，只是按 Android 打包习惯命名为 `libffmpeg.so`。App 会从 `context.applicationInfo.nativeLibraryDir/libffmpeg.so` 通过 `ProcessBuilder` 调用，不经过 shell 拼接，因此支持中文文件名和带空格路径。

建议自编译 FFmpeg 时至少启用 GIF 所需能力和常见输入解码器，并确认命令支持：

```bash
ffmpeg -hide_banner -filters | grep palette
```

## 开发运行

需要安装 Android Studio 或本机 Android SDK，并使用 JDK 17。本机命令行环境可通过 `ANDROID_HOME` 指向 SDK，例如：

```bash
cd android
JAVA_HOME=/path/to/jdk17 ANDROID_HOME=/path/to/android-sdk gradle test
JAVA_HOME=/path/to/jdk17 ANDROID_HOME=/path/to/android-sdk gradle assembleDebug
```

如果使用 Android Studio，直接打开 `android/` 目录并等待 Gradle Sync 完成即可。

```bash
gradle test
gradle assembleDebug
```

## APK 打包

放入 `arm64-v8a/libffmpeg.so` 后：

```bash
cd android
JAVA_HOME=/path/to/jdk17 ANDROID_HOME=/path/to/android-sdk gradle assembleDebug
```

产物位于：

```text
android/app/build/outputs/apk/debug/app-debug.apk
```

## 阶段计划

第二阶段已完成缩略图时间轴、点击定位、起止标记、上一帧/下一帧和逐帧设置起止点。

第三阶段完善取消导出、错误提示、分享体验、缓存清理、大视频加载与低端机性能优化。
