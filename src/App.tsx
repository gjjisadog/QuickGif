import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent, type PointerEvent, type WheelEvent } from 'react'
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { openPath } from '@tauri-apps/plugin-opener'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import {
  ChevronLeft,
  ChevronRight,
  Clapperboard,
  CircleAlert,
  Download,
  FolderOpen,
  Pause,
  Play,
  X,
} from 'lucide-react'
import './App.css'
import { consumeWheelStep, moveFocusRange, wheelSecondsFromDelta, zoomFocusRange } from './trackpadInteraction'

type VideoInfo = {
  path: string
  fileName: string
  duration: number
  width: number
  height: number
  frameRate: number
  codec: string
  totalFrames: number
}

type ExportProgress = {
  progress: number
  stage: string
}

type ExportResult = {
  outputPath: string
  folderPath: string
  frameCount?: number
  imagePath?: string
  videoPath?: string
  contentIdentifier?: string
}

type Language = 'zh-CN' | 'en'
type ExportFormat = 'mp4' | 'webp' | 'gif' | 'livePhoto'
type ShortcutAction =
  | 'playPause'
  | 'previousFrame'
  | 'nextFrame'
  | 'setStartFrame'
  | 'setEndFrame'
  | 'exportClip'

type ShortcutBinding = {
  key: string
  code: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
  shiftKey: boolean
  label: string
}

type KeyboardShortcuts = Record<ShortcutAction, ShortcutBinding>

type AppSettings = {
  language: Language
  defaultExportDir: string | null
  shortcuts: KeyboardShortcuts
}

type ThumbnailResult = {
  directory: string
  files: string[]
  startTime: number
  intervalSeconds: number
}

type FramePoint = {
  frame: number
  timestamp: number
}

const videoFilters = [
  { name: 'Video', extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm'] },
  { name: 'Live Photo', extensions: ['jpg', 'jpeg', 'heic', 'heif', 'png', 'mov', 'mp4'] },
]

const DEFAULT_FOCUS_SECONDS = 12
const MIN_FOCUS_SECONDS = 1
const PREVIEW_WHEEL_STEP_THRESHOLD = 80
const DETAIL_THUMBNAIL_DEBOUNCE_MS = 250
const runningInTauri = isTauri()
const defaultShortcuts: KeyboardShortcuts = {
  playPause: { key: ' ', code: 'Space', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, label: 'Space' },
  previousFrame: { key: 'ArrowLeft', code: 'ArrowLeft', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, label: 'Left' },
  nextFrame: { key: 'ArrowRight', code: 'ArrowRight', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, label: 'Right' },
  setStartFrame: { key: 'i', code: 'KeyI', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, label: 'I' },
  setEndFrame: { key: 'o', code: 'KeyO', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, label: 'O' },
  exportClip: { key: 'e', code: 'KeyE', ctrlKey: true, metaKey: true, altKey: false, shiftKey: false, label: 'Ctrl/Cmd + E' },
}
const shortcutActions: ShortcutAction[] = ['playPause', 'previousFrame', 'nextFrame', 'setStartFrame', 'setEndFrame', 'exportClip']
const defaultSettings: AppSettings = { language: 'zh-CN', defaultExportDir: null, shortcuts: defaultShortcuts }
const menuActions = {
  languageZh: 'quickgif-language-zh',
  languageEn: 'quickgif-language-en',
  setExportDir: 'quickgif-set-export-dir',
  useSourceDir: 'quickgif-use-source-dir',
  exportFormatMp4: 'quickgif-export-format-mp4',
  exportFormatWebp: 'quickgif-export-format-webp',
  exportFormatGif: 'quickgif-export-format-gif',
  exportFormatLivePhoto: 'quickgif-export-format-live-photo',
  keyboardShortcuts: 'quickgif-keyboard-shortcuts',
} as const

const exportFormatMenuMap: Record<string, ExportFormat> = {
  [menuActions.exportFormatMp4]: 'mp4',
  [menuActions.exportFormatWebp]: 'webp',
  [menuActions.exportFormatGif]: 'gif',
  [menuActions.exportFormatLivePhoto]: 'livePhoto',
}

const exportFormatLabel: Record<ExportFormat, string> = {
  mp4: 'MP4',
  webp: 'WebP',
  gif: 'GIF',
  livePhoto: 'Live Photo',
}

const translations = {
  'zh-CN': {
    browserPreviewOnly: '浏览器预览仅用于界面检查；请在 Tauri 桌面应用中选择或拖入视频。',
    browserChooseOnly: '浏览器预览仅用于界面检查；请运行 Tauri 桌面应用选择视频。',
    thumbnailAndFrameIndexing: '正在生成缩略图和帧索引',
    thumbnailFailed: '缩略图生成失败',
    frameIndexFailed: '帧索引生成失败',
    detailThumbnailLoading: '正在生成细节缩略图',
    detailThumbnailFailed: '细节缩略图生成失败',
    submitExport: '提交导出任务',
    paletteGenerating: '生成调色板',
    gifGenerating: '生成 GIF',
    mp4Preparing: '准备导出 MP4',
    mp4Encoding: '生成 MP4',
    mp4Done: 'MP4 导出完成',
    webpPreparing: '准备导出 WebP',
    webpEncoding: '生成 WebP',
    webpDone: 'WebP 导出完成',
    finalizingExport: '收尾保存',
    exportDone: '导出完成',
    exportCurrentFrame: '导出当前帧图片',
    imageExportDone: '图片导出完成',
    exportFrameSequence: '逐帧导出选区',
    frameSequenceExporting: '正在逐帧导出图片',
    frameSequenceDone: '逐帧图片导出完成',
    exportedFrames: '导出帧数',
    preview: '预览',
    unloadedVideo: '未载入视频',
    exportMp4: '导出 MP4',
    exportWebp: '导出 WebP',
    exportGif: '导出 GIF',
    exportLivePhoto: '导出实况照片',
    livePhotoPairHint: '已导出配对 JPG + MOV，可一并导入「照片」App',
    livePhotoDurationHint: '实况照片选区需在 0.5–6 秒',
    livePhotoPreparing: '准备导出实况照片',
    livePhotoEncodingVideo: '生成实况视频',
    livePhotoExtractingStill: '提取关键静帧',
    livePhotoWritingMetadata: '写入配对元数据',
    livePhotoDone: '实况照片导出完成',
    chooseVideo: '选择视频',
    loadingVideoInfo: '正在读取视频信息',
    chooseOrDropVideo: '选择或拖入视频',
    positioningTimeline: '定位时间轴',
    generatingPositionThumbnails: '正在生成定位缩略图',
    waitingImport: '等待导入视频',
    frameIndex: '帧索引',
    overview: '总览',
    dragGreenBox: '拖动绿色框快速定位',
    trackpadOverviewHint: '双指横滑移动 · 捏合缩放',
    dragSelection: '拖动选区',
    waitingOverview: '正在等待总览缩略图',
    dropVideoForOverview: '拖入视频后显示总览时间轴',
    detail: '细节',
    generating: '生成中',
    waitingSelection: '等待选区',
    waitingDetail: '正在等待细节缩略图',
    overviewSelectionForDetail: '总览选区后显示细节',
    trackpadDetailHint: '双指横滑精确定位 · 捏合缩放',
    openFolder: '打开文件夹',
    operationIncomplete: '操作未完成',
    closeError: '关闭错误提示',
    setStartFrame: '设为开始帧',
    setEndFrame: '设为结束帧',
    exportCurrentFrameImage: '导出当前帧图片',
    exportPngShort: '导出 PNG',
    clipLength: '片段',
    currentFormat: '格式',
    playPause: 'Space 播放 / 暂停',
    previousFrame: '上一帧',
    nextFrame: '下一帧',
    hideControls: '隐藏播放控件',
    showControls: '显示播放控件',
    settingExportFolder: '设置默认导出文件夹',
    sourceFolderRestored: '已恢复为原视频目录导出',
    settingsSaveFailed: '设置保存失败',
    exportFormatSelected: '导出格式已切换',
    keyboardShortcuts: '快捷键',
    resetShortcuts: '恢复默认',
    closeShortcuts: '关闭快捷键设置',
    recordingShortcut: '按下新快捷键',
    shortcutSaved: '快捷键已保存',
    playPauseAction: '播放 / 暂停',
    previousFrameAction: '上一帧',
    nextFrameAction: '下一帧',
    setStartFrameAction: '设为开始帧',
    setEndFrameAction: '设为结束帧',
    exportClipAction: '导出当前片段',
  },
  en: {
    browserPreviewOnly: 'Browser preview is only for UI checks. Choose or drop a video in the Tauri desktop app.',
    browserChooseOnly: 'Browser preview is only for UI checks. Please choose a video in the Tauri desktop app.',
    thumbnailAndFrameIndexing: 'Generating thumbnails and frame index',
    thumbnailFailed: 'Thumbnail generation failed',
    frameIndexFailed: 'Frame indexing failed',
    detailThumbnailLoading: 'Generating detail thumbnails',
    detailThumbnailFailed: 'Detail thumbnail generation failed',
    submitExport: 'Submitting export',
    paletteGenerating: 'Generating palette',
    gifGenerating: 'Generating GIF',
    mp4Preparing: 'Preparing MP4 export',
    mp4Encoding: 'Encoding MP4',
    mp4Done: 'MP4 export complete',
    webpPreparing: 'Preparing WebP export',
    webpEncoding: 'Encoding WebP',
    webpDone: 'WebP export complete',
    finalizingExport: 'Finalizing export',
    exportDone: 'Export complete',
    exportCurrentFrame: 'Exporting current frame image',
    imageExportDone: 'Image export complete',
    exportFrameSequence: 'Export Range as Frames',
    frameSequenceExporting: 'Exporting frame images',
    frameSequenceDone: 'Frame image export complete',
    exportedFrames: 'Frames exported',
    preview: 'Preview',
    unloadedVideo: 'No video loaded',
    exportMp4: 'Export MP4',
    exportWebp: 'Export WebP',
    exportGif: 'Export GIF',
    exportLivePhoto: 'Export Live Photo',
    livePhotoPairHint: 'Exported paired JPG + MOV — import both into Photos',
    livePhotoDurationHint: 'Live Photo range must be 0.5–6 seconds',
    livePhotoPreparing: 'Preparing Live Photo export',
    livePhotoEncodingVideo: 'Encoding Live Photo video',
    livePhotoExtractingStill: 'Extracting key still',
    livePhotoWritingMetadata: 'Writing pairing metadata',
    livePhotoDone: 'Live Photo export complete',
    chooseVideo: 'Choose Video',
    loadingVideoInfo: 'Reading video info',
    chooseOrDropVideo: 'Choose or drop a video',
    positioningTimeline: 'Positioning Timeline',
    generatingPositionThumbnails: 'Generating positioning thumbnails',
    waitingImport: 'Waiting for video',
    frameIndex: 'frame index',
    overview: 'Overview',
    dragGreenBox: 'Drag the green box to locate quickly',
    trackpadOverviewHint: 'Two-finger swipe to move · pinch to zoom',
    dragSelection: 'Drag selection',
    waitingOverview: 'Waiting for overview thumbnails',
    dropVideoForOverview: 'Drop a video to show the overview timeline',
    detail: 'Detail',
    generating: 'Generating',
    waitingSelection: 'Waiting for selection',
    waitingDetail: 'Waiting for detail thumbnails',
    overviewSelectionForDetail: 'Select a range in overview to show detail',
    trackpadDetailHint: 'Two-finger swipe to scrub · pinch to zoom',
    openFolder: 'Open Folder',
    operationIncomplete: 'Operation incomplete',
    closeError: 'Close error',
    setStartFrame: 'Set Start Frame',
    setEndFrame: 'Set End Frame',
    exportCurrentFrameImage: 'Export Current Frame Image',
    exportPngShort: 'Export PNG',
    clipLength: 'Clip',
    currentFormat: 'Format',
    playPause: 'Space Play / Pause',
    previousFrame: 'Previous Frame',
    nextFrame: 'Next Frame',
    hideControls: 'Hide playback controls',
    showControls: 'Show playback controls',
    settingExportFolder: 'Setting default export folder',
    sourceFolderRestored: 'Export folder reset to source video folder',
    settingsSaveFailed: 'Failed to save settings',
    exportFormatSelected: 'Export format selected',
    keyboardShortcuts: 'Keyboard Shortcuts',
    resetShortcuts: 'Reset Defaults',
    closeShortcuts: 'Close keyboard shortcut settings',
    recordingShortcut: 'Press new shortcut',
    shortcutSaved: 'Shortcut saved',
    playPauseAction: 'Play / Pause',
    previousFrameAction: 'Previous Frame',
    nextFrameAction: 'Next Frame',
    setStartFrameAction: 'Set Start Frame',
    setEndFrameAction: 'Set End Frame',
    exportClipAction: 'Export Current Clip',
  },
} satisfies Record<Language, Record<string, string>>

const progressStageTranslations: Record<string, keyof typeof translations['zh-CN']> = {
  准备导出: 'submitExport',
  生成调色板: 'paletteGenerating',
  '生成 GIF': 'gifGenerating',
  收尾保存: 'finalizingExport',
  导出完成: 'exportDone',
  提交导出任务: 'submitExport',
  导出当前帧图片: 'exportCurrentFrame',
  图片导出完成: 'imageExportDone',
  frameSequenceExporting: 'frameSequenceExporting',
  frameSequenceDone: 'frameSequenceDone',
  mp4Preparing: 'mp4Preparing',
  mp4Encoding: 'mp4Encoding',
  mp4Done: 'mp4Done',
  webpPreparing: 'webpPreparing',
  webpEncoding: 'webpEncoding',
  webpDone: 'webpDone',
  livePhotoPreparing: 'livePhotoPreparing',
  livePhotoEncodingVideo: 'livePhotoEncodingVideo',
  livePhotoExtractingStill: 'livePhotoExtractingStill',
  livePhotoWritingMetadata: 'livePhotoWritingMetadata',
  livePhotoDone: 'livePhotoDone',
}

const exportButtonKey: Record<ExportFormat, keyof typeof translations['zh-CN']> = {
  mp4: 'exportMp4',
  webp: 'exportWebp',
  gif: 'exportGif',
  livePhoto: 'exportLivePhoto',
}

const shortcutActionLabelKey: Record<ShortcutAction, keyof typeof translations['zh-CN']> = {
  playPause: 'playPauseAction',
  previousFrame: 'previousFrameAction',
  nextFrame: 'nextFrameAction',
  setStartFrame: 'setStartFrameAction',
  setEndFrame: 'setEndFrameAction',
  exportClip: 'exportClipAction',
}

function App() {
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const backdropVideoRef = useRef<HTMLVideoElement | null>(null)
  const overviewTrackRef = useRef<HTMLDivElement | null>(null)
  const detailTrackRef = useRef<HTMLDivElement | null>(null)
  const previewWheelAccumulatorRef = useRef(0)
  const loadRequestRef = useRef(0)
  const detailRequestRef = useRef(0)
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null)
  const [videoPath, setVideoPath] = useState('')
  const [overviewThumbnails, setOverviewThumbnails] = useState<ThumbnailResult | null>(null)
  const [detailThumbnails, setDetailThumbnails] = useState<ThumbnailResult | null>(null)
  const [frames, setFrames] = useState<FramePoint[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [isIndexing, setIsIndexing] = useState(false)
  const [isDetailLoading, setIsDetailLoading] = useState(false)
  const [isDragging, setIsDragging] = useState(false)
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [focusStart, setFocusStart] = useState(0)
  const [focusEnd, setFocusEnd] = useState(0)
  const [startTime, setStartTime] = useState(0)
  const [endTime, setEndTime] = useState(0)
  const [error, setError] = useState('')
  const [timelineMessage, setTimelineMessage] = useState('')
  const [detailMessage, setDetailMessage] = useState('')
  const [exportProgress, setExportProgress] = useState<ExportProgress | null>(null)
  const [exportResult, setExportResult] = useState<ExportResult | null>(null)
  const [exportFormat, setExportFormat] = useState<ExportFormat>('mp4')
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null)
  const [shortcutModalOpen, setShortcutModalOpen] = useState(false)
  const [recordingShortcut, setRecordingShortcut] = useState<ShortcutAction | null>(null)
  const [settings, setSettings] = useState<AppSettings>(defaultSettings)
  const language = settings.language
  const t = useCallback((key: keyof typeof translations['zh-CN']) => translations[language][key], [language])
  const statusText = useCallback(
    (value: string) => {
      const key = progressStageTranslations[value] ?? (value as keyof typeof translations['zh-CN'])
      return translations[language][key] ?? value
    },
    [language],
  )

  const applySettings = useCallback((nextSettings: AppSettings) => {
    setSettings({
      language: nextSettings.language === 'en' ? 'en' : 'zh-CN',
      defaultExportDir: nextSettings.defaultExportDir ?? null,
      shortcuts: normalizeShortcuts(nextSettings.shortcuts),
    })
  }, [])

  const frameDuration = useMemo(() => {
    const fps = videoInfo?.frameRate || 25
    return 1 / fps
  }, [videoInfo])

  const videoSrc = useMemo(() => {
    if (!videoPath) {
      return ''
    }
    return runningInTauri ? convertFileSrc(videoPath) : videoPath
  }, [videoPath])

  const overviewThumbnailItems = useMemo(() => buildThumbnailItems(overviewThumbnails), [overviewThumbnails])
  const detailThumbnailItems = useMemo(() => buildThumbnailItems(detailThumbnails), [detailThumbnails])

  const loadVideo = useCallback(async (path: string) => {
    if (!runningInTauri) {
      setError(t('browserPreviewOnly'))
      return
    }

    const requestId = loadRequestRef.current + 1
    loadRequestRef.current = requestId
    setIsLoading(true)
    setIsIndexing(false)
    setError('')
    setTimelineMessage('')
    setDetailMessage('')
    setExportResult(null)
    setExportProgress(null)
    setVideoInfo(null)
    setVideoPath('')
    setOverviewThumbnails(null)
    setDetailThumbnails(null)
    setFrames([])

    try {
      const info = await invoke<VideoInfo>('probe_video_command', { path })
      if (loadRequestRef.current !== requestId) {
        return
      }
      setVideoInfo(info)
      setVideoPath(info.path)
      setCurrentTime(0)
      setFocusStart(0)
      setFocusEnd(Math.min(DEFAULT_FOCUS_SECONDS, info.duration))
      setStartTime(0)
      setEndTime(Math.min(DEFAULT_FOCUS_SECONDS, info.duration))
      setIsPlaying(false)
      setIsLoading(false)

      setIsIndexing(true)
      setTimelineMessage('thumbnailAndFrameIndexing')
      const [thumbnailResult, frameResult] = await Promise.allSettled([
        invoke<ThumbnailResult>('generate_thumbnails_command', {
          path: info.path,
          duration: info.duration,
        }),
        invoke<FramePoint[]>('build_frame_index_command', { path: info.path }),
      ])

      if (loadRequestRef.current !== requestId) {
        return
      }

      const messages: string[] = []
      if (thumbnailResult.status === 'fulfilled') {
        setOverviewThumbnails(thumbnailResult.value)
      } else {
        messages.push(`${t('thumbnailFailed')}：${String(thumbnailResult.reason)}`)
      }

      if (frameResult.status === 'fulfilled') {
        setFrames(frameResult.value)
      } else {
        messages.push(`${t('frameIndexFailed')}：${String(frameResult.reason)}`)
      }

      setTimelineMessage(messages.length > 0 ? messages.join('；') : '')
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught))
    } finally {
      setIsLoading(false)
      if (loadRequestRef.current === requestId) {
        setIsIndexing(false)
      }
    }
  }, [t])

  useEffect(() => {
    if (!runningInTauri || !videoInfo || focusEnd <= focusStart) {
      return
    }

    const requestId = detailRequestRef.current + 1
    detailRequestRef.current = requestId
    setIsDetailLoading(true)
    setDetailThumbnails(null)
    setDetailMessage('detailThumbnailLoading')

    const timer = window.setTimeout(() => {
      void invoke<ThumbnailResult>('generate_detail_thumbnails_command', {
        path: videoInfo.path,
        startTime: focusStart,
        duration: focusEnd - focusStart,
      })
        .then((result) => {
          if (detailRequestRef.current !== requestId) {
            return
          }
          setDetailThumbnails(result)
          setDetailMessage('')
        })
        .catch((caught) => {
          if (detailRequestRef.current !== requestId) {
            return
          }
          setDetailMessage(`${t('detailThumbnailFailed')}：${String(caught)}`)
        })
        .finally(() => {
          if (detailRequestRef.current === requestId) {
            setIsDetailLoading(false)
          }
        })
    }, DETAIL_THUMBNAIL_DEBOUNCE_MS)

    return () => window.clearTimeout(timer)
  }, [focusEnd, focusStart, t, videoInfo])

  const chooseVideo = useCallback(async () => {
    if (!runningInTauri) {
      setError(t('browserChooseOnly'))
      return
    }

    const selected = await open({
      multiple: false,
      directory: false,
      filters: videoFilters,
    })

    if (typeof selected === 'string') {
      await loadVideo(selected)
    }
  }, [loadVideo, t])

  const syncBackdropVideo = useCallback((source: HTMLVideoElement) => {
    const backdrop = backdropVideoRef.current
    if (!backdrop) {
      return
    }

    if (Math.abs(backdrop.currentTime - source.currentTime) > 0.25) {
      backdrop.currentTime = source.currentTime
    }

    if (source.paused) {
      backdrop.pause()
      return
    }

    if (backdrop.paused) {
      void backdrop.play().catch(() => undefined)
    }
  }, [])

  const togglePlayback = useCallback(async () => {
    const video = videoRef.current
    const backdrop = backdropVideoRef.current
    if (!video) {
      return
    }

    if (video.paused) {
      await video.play()
      if (backdrop) {
        backdrop.currentTime = video.currentTime
        void backdrop.play().catch(() => undefined)
      }
      setIsPlaying(true)
    } else {
      video.pause()
      backdrop?.pause()
      setIsPlaying(false)
    }
  }, [])

  const seekToTime = useCallback(
    (time: number, snapToFrame = true) => {
      const video = videoRef.current
      if (!video || !videoInfo) {
        return
      }
      video.pause()
      setIsPlaying(false)
      const safeTime = clamp(time, 0, videoInfo.duration)
      const nextTime =
        snapToFrame && frames.length > 0 ? frames[nearestFrameIndex(frames, safeTime)].timestamp : safeTime
      video.currentTime = nextTime
      const backdrop = backdropVideoRef.current
      if (backdrop) {
        backdrop.pause()
        backdrop.currentTime = nextTime
      }
      setCurrentTime(nextTime)
    },
    [frames, videoInfo],
  )

  const stepFrame = useCallback(
    (direction: -1 | 1) => {
      const video = videoRef.current
      if (!video || !videoInfo) {
        return
      }

      if (frames.length === 0) {
        seekToTime(video.currentTime + direction * frameDuration, false)
        return
      }

      const targetIndex = adjacentFrameIndex(frames, video.currentTime, direction)
      seekToTime(frames[targetIndex].timestamp, false)
    },
    [frameDuration, frames, seekToTime, videoInfo],
  )

  const handlePreviewWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      if (!videoInfo || Math.abs(event.deltaX) <= Math.abs(event.deltaY)) {
        return
      }

      event.preventDefault()
      const result = consumeWheelStep({
        accumulator: previewWheelAccumulatorRef.current,
        delta: event.deltaX,
        threshold: PREVIEW_WHEEL_STEP_THRESHOLD,
      })
      previewWheelAccumulatorRef.current = result.accumulator
      if (result.steps !== 0) {
        stepFrame(result.steps > 0 ? 1 : -1)
      }
    },
    [stepFrame, videoInfo],
  )

  const setMarker = useCallback(
    (marker: 'start' | 'end') => {
      if (!videoInfo) {
        return
      }

      const markerTime = frames.length > 0 ? frames[nearestFrameIndex(frames, currentTime)].timestamp : currentTime

      if (marker === 'start') {
        const nextStart = clamp(markerTime, 0, Math.max(0, endTime - frameDuration))
        setStartTime(nextStart)
      } else {
        const nextEnd = clamp(markerTime, Math.min(videoInfo.duration, startTime + frameDuration), videoInfo.duration)
        setEndTime(nextEnd)
      }
    },
    [currentTime, endTime, frameDuration, frames, startTime, videoInfo],
  )

  const saveShortcuts = useCallback(
    async (shortcuts: KeyboardShortcuts) => {
      const nextSettings = await invoke<AppSettings>('set_shortcuts_command', { shortcuts })
      applySettings(nextSettings)
      setTimelineMessage('shortcutSaved')
    },
    [applySettings],
  )

  const updateShortcut = useCallback(
    async (action: ShortcutAction, binding: ShortcutBinding) => {
      try {
        await saveShortcuts({ ...settings.shortcuts, [action]: binding })
        setRecordingShortcut(null)
      } catch (caught) {
        setError(`${t('settingsSaveFailed')}：${caught instanceof Error ? caught.message : String(caught)}`)
      }
    },
    [saveShortcuts, settings.shortcuts, t],
  )

  const resetShortcuts = useCallback(async () => {
    try {
      await saveShortcuts(defaultShortcuts)
      setRecordingShortcut(null)
    } catch (caught) {
      setError(`${t('settingsSaveFailed')}：${caught instanceof Error ? caught.message : String(caught)}`)
    }
  }, [saveShortcuts, t])

  const seekFromDetailTimeline = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (!videoInfo) {
        return
      }
      const rect = event.currentTarget.getBoundingClientRect()
      const ratio = clamp((event.clientX - rect.left) / rect.width, 0, 1)
      seekToTime(focusStart + (focusEnd - focusStart) * ratio)
    },
    [focusEnd, focusStart, seekToTime, videoInfo],
  )

  const handleDetailWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      if (!videoInfo) {
        return
      }

      const rect = event.currentTarget.getBoundingClientRect()
      if (event.ctrlKey) {
        event.preventDefault()
        const anchorRatio = clamp((event.clientX - rect.left) / rect.width, 0, 1)
        const anchor = focusStart + (focusEnd - focusStart) * anchorRatio
        const [nextStart, nextEnd] = zoomFocusRange({
          start: focusStart,
          end: focusEnd,
          duration: videoInfo.duration,
          anchor,
          deltaY: event.deltaY,
          minSpan: MIN_FOCUS_SECONDS,
        })
        setFocusStart(nextStart)
        setFocusEnd(nextEnd)
        return
      }

      const deltaSeconds = wheelSecondsFromDelta({
        deltaX: event.deltaX,
        deltaY: event.deltaY,
        trackPixels: rect.width,
        visibleSeconds: focusEnd - focusStart,
      })
      if (deltaSeconds !== 0) {
        event.preventDefault()
        seekToTime(currentTime + deltaSeconds)
      }
    },
    [currentTime, focusEnd, focusStart, seekToTime, videoInfo],
  )

  const commitFocusRange = useCallback(
    (nextStart: number, nextEnd: number) => {
      if (!videoInfo) {
        return
      }
      const [safeStart, safeEnd] = clampFocusRange(nextStart, nextEnd, videoInfo.duration)
      setFocusStart(safeStart)
      setFocusEnd(safeEnd)
      seekToTime(focusCenterTime(safeStart, safeEnd))
    },
    [seekToTime, videoInfo],
  )

  const centerFocusAt = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (!videoInfo) {
        return
      }
      const target = event.target as HTMLElement
      if (target.closest('[data-focus-control]')) {
        return
      }
      const rect = event.currentTarget.getBoundingClientRect()
      const time = videoInfo.duration * clamp((event.clientX - rect.left) / rect.width, 0, 1)
      const length = Math.max(MIN_FOCUS_SECONDS, focusEnd - focusStart)
      commitFocusRange(time - length / 2, time + length / 2)
    },
    [commitFocusRange, focusEnd, focusStart, videoInfo],
  )

  const handleOverviewWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      if (!videoInfo) {
        return
      }

      const rect = event.currentTarget.getBoundingClientRect()
      if (event.ctrlKey) {
        event.preventDefault()
        const anchor = videoInfo.duration * clamp((event.clientX - rect.left) / rect.width, 0, 1)
        const [nextStart, nextEnd] = zoomFocusRange({
          start: focusStart,
          end: focusEnd,
          duration: videoInfo.duration,
          anchor,
          deltaY: event.deltaY,
          minSpan: MIN_FOCUS_SECONDS,
        })
        setFocusStart(nextStart)
        setFocusEnd(nextEnd)
        seekToTime(focusCenterTime(nextStart, nextEnd))
        return
      }

      const deltaSeconds = wheelSecondsFromDelta({
        deltaX: event.deltaX,
        deltaY: event.deltaY,
        trackPixels: rect.width,
        visibleSeconds: videoInfo.duration,
      })
      if (deltaSeconds !== 0) {
        event.preventDefault()
        const [nextStart, nextEnd] = moveFocusRange({
          start: focusStart,
          end: focusEnd,
          duration: videoInfo.duration,
          deltaSeconds,
        })
        setFocusStart(nextStart)
        setFocusEnd(nextEnd)
        seekToTime(focusCenterTime(nextStart, nextEnd))
      }
    },
    [focusEnd, focusStart, seekToTime, videoInfo],
  )

  const beginFocusDrag = useCallback(
    (event: PointerEvent<HTMLElement>, mode: 'move' | 'start' | 'end') => {
      if (!videoInfo || !overviewTrackRef.current) {
        return
      }
      event.preventDefault()
      event.stopPropagation()

      const rect = overviewTrackRef.current.getBoundingClientRect()
      const initialX = event.clientX
      const initialStart = focusStart
      const initialEnd = focusEnd
      let nextStart = initialStart
      let nextEnd = initialEnd

      const applyRange = (clientX: number) => {
        const delta = ((clientX - initialX) / rect.width) * videoInfo.duration
        if (mode === 'move') {
          ;[nextStart, nextEnd] = clampFocusRange(initialStart + delta, initialEnd + delta, videoInfo.duration)
        }
        if (mode === 'start') {
          ;[nextStart, nextEnd] = clampFocusRange(initialStart + delta, initialEnd, videoInfo.duration)
        }
        if (mode === 'end') {
          ;[nextStart, nextEnd] = clampFocusRange(initialStart, initialEnd + delta, videoInfo.duration)
        }
        setFocusStart(nextStart)
        setFocusEnd(nextEnd)
      }

      const handlePointerMove = (moveEvent: globalThis.PointerEvent) => {
        applyRange(moveEvent.clientX)
      }

      const handlePointerUp = () => {
        window.removeEventListener('pointermove', handlePointerMove)
        window.removeEventListener('pointerup', handlePointerUp)
        seekToTime(focusCenterTime(nextStart, nextEnd))
      }

      window.addEventListener('pointermove', handlePointerMove)
      window.addEventListener('pointerup', handlePointerUp, { once: true })
    },
    [focusEnd, focusStart, seekToTime, videoInfo],
  )

  const exportClip = useCallback(async () => {
    if (!videoInfo || startTime >= endTime) {
      return
    }

    if (exportFormat === 'livePhoto') {
      const duration = endTime - startTime
      if (duration < 0.5 || duration > 6) {
        setError(t('livePhotoDurationHint'))
        return
      }
    }

    setError('')
    setExportResult(null)
    setExportProgress({ progress: 1, stage: 'submitExport' })

    try {
      const baseRequest = {
        inputPath: videoInfo.path,
        startTime,
        endTime,
      }

      let command = 'export_mp4_command'
      let request: Record<string, unknown> = baseRequest
      let doneStage = 'mp4Done'

      if (exportFormat === 'gif') {
        command = 'export_gif_command'
        request = {
          ...baseRequest,
          width: 'w720',
          frameRate: 'fps15',
          quality: 'standard',
        }
        doneStage = '导出完成'
      } else if (exportFormat === 'webp') {
        command = 'export_webp_command'
        doneStage = 'webpDone'
      } else if (exportFormat === 'livePhoto') {
        command = 'export_live_photo_command'
        const stillInRange = currentTime >= startTime && currentTime <= endTime
        request = {
          ...baseRequest,
          stillTime: stillInRange ? currentTime : (startTime + endTime) / 2,
        }
        doneStage = 'livePhotoDone'
      }

      const result = await invoke<ExportResult>(command, { request })
      setExportResult(result)
      setExportProgress({
        progress: 100,
        stage: doneStage,
      })
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught))
      setExportProgress(null)
    }
  }, [currentTime, endTime, exportFormat, startTime, t, videoInfo])

  const runShortcutAction = useCallback(
    (action: ShortcutAction) => {
      if (action === 'playPause') {
        void togglePlayback()
        return
      }
      if (action === 'previousFrame') {
        stepFrame(-1)
        return
      }
      if (action === 'nextFrame') {
        stepFrame(1)
        return
      }
      if (action === 'setStartFrame') {
        setMarker('start')
        return
      }
      if (action === 'setEndFrame') {
        setMarker('end')
        return
      }
      if (action === 'exportClip') {
        void exportClip()
      }
    },
    [exportClip, setMarker, stepFrame, togglePlayback],
  )

  const exportCurrentFrameImage = useCallback(async () => {
    if (!videoInfo) {
      return
    }

    setContextMenu(null)
    setError('')
    setExportResult(null)
    setExportProgress({ progress: 1, stage: 'exportCurrentFrame' })

    try {
      const timestamp = frames.length > 0 ? frames[nearestFrameIndex(frames, currentTime)].timestamp : currentTime
      const result = await invoke<ExportResult>('export_frame_image_command', {
        request: {
          inputPath: videoInfo.path,
          timestamp,
        },
      })
      setExportResult(result)
      setExportProgress({ progress: 100, stage: 'imageExportDone' })
      window.setTimeout(() => {
        setExportProgress(null)
      }, 600)
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught))
      setExportProgress(null)
    }
  }, [currentTime, frames, videoInfo])

  const exportFrameSequence = useCallback(async () => {
    if (!videoInfo || startTime >= endTime) {
      return
    }

    setContextMenu(null)
    setError('')
    setExportResult(null)
    setExportProgress({ progress: 1, stage: 'frameSequenceExporting' })

    try {
      let sequenceStart = startTime
      let sequenceEnd = endTime
      if (frames.length > 0) {
        const startIndex = nearestFrameIndex(frames, startTime)
        const endIndex = nearestFrameIndex(frames, endTime)
        sequenceStart = frames[startIndex].timestamp
        sequenceEnd = frames[endIndex + 1]?.timestamp ?? videoInfo.duration
        if (sequenceEnd <= sequenceStart) {
          sequenceEnd = Math.min(videoInfo.duration, sequenceStart + frameDuration)
        }
      }

      const result = await invoke<ExportResult>('export_frame_sequence_command', {
        request: {
          inputPath: videoInfo.path,
          startTime: sequenceStart,
          endTime: sequenceEnd,
        },
      })
      setExportResult(result)
      setExportProgress({ progress: 100, stage: 'frameSequenceDone' })
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught))
      setExportProgress(null)
    }
  }, [endTime, frameDuration, frames, startTime, videoInfo])

  const openContextMenu = useCallback(
    (event: MouseEvent<HTMLElement>) => {
      if (!videoInfo) {
        return
      }
      event.preventDefault()
      setContextMenu({ x: event.clientX, y: event.clientY })
    },
    [videoInfo],
  )

  const setMarkerFromContextMenu = useCallback(
    (marker: 'start' | 'end') => {
      setMarker(marker)
      setContextMenu(null)
    },
    [setMarker],
  )

  const handleMenuAction = useCallback(
    async (action: string) => {
      try {
        if (action === menuActions.languageZh || action === menuActions.languageEn) {
          const nextLanguage = action === menuActions.languageEn ? 'en' : 'zh-CN'
          const nextSettings = await invoke<AppSettings>('set_language_command', { language: nextLanguage })
          applySettings(nextSettings)
          return
        }

        if (action === menuActions.setExportDir) {
          const selected = await open({
            multiple: false,
            directory: true,
          })
          if (typeof selected === 'string') {
            const nextSettings = await invoke<AppSettings>('set_default_export_dir_command', {
              defaultExportDir: selected,
            })
            applySettings(nextSettings)
          }
          return
        }

        if (action === menuActions.useSourceDir) {
          const nextSettings = await invoke<AppSettings>('set_default_export_dir_command', {
            defaultExportDir: null,
          })
          applySettings(nextSettings)
          setTimelineMessage('sourceFolderRestored')
          return
        }

        const nextExportFormat = exportFormatMenuMap[action]
        if (nextExportFormat) {
          setExportFormat(nextExportFormat)
          setTimelineMessage(`${t('exportFormatSelected')}：${exportFormatLabel[nextExportFormat]}`)
          return
        }

        if (action === menuActions.keyboardShortcuts) {
          setShortcutModalOpen(true)
          setRecordingShortcut(null)
        }
      } catch (caught) {
        setError(`${t('settingsSaveFailed')}：${caught instanceof Error ? caught.message : String(caught)}`)
      }
    },
    [applySettings, t],
  )

  useEffect(() => {
    if (!runningInTauri) {
      return
    }

    void invoke<AppSettings>('get_settings_command')
      .then(applySettings)
      .catch((caught) => {
        setError(`${t('settingsSaveFailed')}：${caught instanceof Error ? caught.message : String(caught)}`)
      })
  }, [applySettings, t])

  useEffect(() => {
    if (!runningInTauri) {
      return
    }

    const unlistenPromise = listen<string>('quickgif-menu', (event) => {
      void handleMenuAction(event.payload)
    })
    return () => {
      void unlistenPromise.then((unlisten) => unlisten())
    }
  }, [handleMenuAction])

  useEffect(() => {
    if (!runningInTauri) {
      return
    }

    const unlistenPromise = Promise.all([
      listen<ExportProgress>('mp4-export-progress', (event) => {
        setExportProgress(event.payload)
      }),
      listen<ExportProgress>('webp-export-progress', (event) => {
        setExportProgress(event.payload)
      }),
      listen<ExportProgress>('gif-export-progress', (event) => {
        setExportProgress(event.payload)
      }),
      listen<ExportProgress>('livephoto-export-progress', (event) => {
        setExportProgress(event.payload)
      }),
    ])
    return () => {
      void unlistenPromise.then((unlisteners) => {
        unlisteners.forEach((unlisten) => unlisten())
      })
    }
  }, [])

  useEffect(() => {
    if (!runningInTauri) {
      return
    }

    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        setIsDragging(true)
      }
      if (event.payload.type === 'leave') {
        setIsDragging(false)
      }
      if (event.payload.type === 'drop') {
        setIsDragging(false)
        const [path] = event.payload.paths
        if (path) {
          void loadVideo(path)
        }
      }
    })

    return () => {
      void unlistenPromise.then((unlisten) => unlisten())
    }
  }, [loadVideo])

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      if (recordingShortcut) {
        event.preventDefault()
        if (event.key === 'Escape') {
          setRecordingShortcut(null)
          return
        }
        const nextShortcut = shortcutFromEvent(event)
        if (nextShortcut) {
          void updateShortcut(recordingShortcut, nextShortcut)
        }
        return
      }

      if (shortcutModalOpen) {
        if (event.key === 'Escape') {
          setShortcutModalOpen(false)
        }
        return
      }

      if (target?.matches('input, select, textarea')) {
        return
      }

      const matchedAction = shortcutActions.find((action) => shortcutMatches(settings.shortcuts[action], event))
      if (matchedAction) {
        event.preventDefault()
        runShortcutAction(matchedAction)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [recordingShortcut, runShortcutAction, settings.shortcuts, shortcutModalOpen, updateShortcut])

  useEffect(() => {
    if (!contextMenu) {
      return
    }

    const closeContextMenu = () => setContextMenu(null)
    window.addEventListener('click', closeContextMenu)
    window.addEventListener('keydown', closeContextMenu)
    return () => {
      window.removeEventListener('click', closeContextMenu)
      window.removeEventListener('keydown', closeContextMenu)
    }
  }, [contextMenu])

  const canExport = Boolean(videoInfo && startTime < endTime && !exportProgress)
  const focusPercent = videoInfo?.duration ? (focusStart / videoInfo.duration) * 100 : 0
  const focusWidthPercent = videoInfo?.duration ? ((focusEnd - focusStart) / videoInfo.duration) * 100 : 0
  const detailDuration = Math.max(focusEnd - focusStart, MIN_FOCUS_SECONDS)
  const detailStartPercent = clamp(((startTime - focusStart) / detailDuration) * 100, 0, 100)
  const detailEndPercent = clamp(((endTime - focusStart) / detailDuration) * 100, 0, 100)
  const detailRangeWidthPercent = Math.max(0, detailEndPercent - detailStartPercent)
  const detailPlayheadPercent = clamp(((currentTime - focusStart) / detailDuration) * 100, 0, 100)
  const overviewPlayheadPercent = videoInfo?.duration ? (currentTime / videoInfo.duration) * 100 : 0
  const clipDuration = Math.max(0, endTime - startTime)
  const activeDetailThumbnail = detailThumbnails?.intervalSeconds
    ? Math.max(0, Math.floor((currentTime - detailThumbnails.startTime) / detailThumbnails.intervalSeconds))
    : -1

  return (
    <main className={`app-shell ${isDragging ? 'is-dragging' : ''}`}>
      <div className="workbench">
        <section className="stage-column">
          <section className="preview-panel">
            <div className="panel-title">
              <span>{t('preview')}</span>
              <strong>{videoInfo ? videoInfo.fileName : t('unloadedVideo')}</strong>
              {videoInfo && (
                <div className="panel-title-actions">
                  <button
                    type="button"
                    className="export-button compact-export-button"
                    onClick={exportClip}
                    disabled={!canExport}
                    title={`Ctrl/Cmd + E ${t(exportButtonKey[exportFormat])}`}
                  >
                    <Download size={17} />
                    {t(exportButtonKey[exportFormat])}
                  </button>
                </div>
              )}
            </div>
            {videoInfo ? (
              <div className="preview-stage" onContextMenu={openContextMenu} onWheel={handlePreviewWheel}>
                <video
                  ref={backdropVideoRef}
                  className="video-backdrop"
                  src={videoSrc}
                  muted
                  playsInline
                  aria-hidden="true"
                  tabIndex={-1}
                />
                <video
                  ref={videoRef}
                  className="video-preview"
                  src={videoSrc}
                  onTimeUpdate={(event) => {
                    setCurrentTime(event.currentTarget.currentTime)
                    syncBackdropVideo(event.currentTarget)
                  }}
                  onSeeked={(event) => {
                    setCurrentTime(event.currentTarget.currentTime)
                    syncBackdropVideo(event.currentTarget)
                  }}
                  onPlay={(event) => {
                    setIsPlaying(true)
                    syncBackdropVideo(event.currentTarget)
                  }}
                  onPause={() => {
                    setIsPlaying(false)
                    backdropVideoRef.current?.pause()
                  }}
                  controls={false}
                />
                <div className="floating-preview-controls">
                  <button type="button" className="icon-button floating-control-button floating-play-button" onClick={togglePlayback} title={t('playPause')}>
                    {isPlaying ? <Pause size={20} /> : <Play size={20} />}
                  </button>
                  <button type="button" className="icon-button floating-control-button" onClick={() => stepFrame(-1)} title={t('previousFrame')}>
                    <ChevronLeft size={21} />
                  </button>
                  <button type="button" className="icon-button floating-control-button" onClick={() => stepFrame(1)} title={t('nextFrame')}>
                    <ChevronRight size={21} />
                  </button>
                </div>
              </div>
            ) : (
              <button type="button" className="drop-zone" onClick={chooseVideo}>
                <Clapperboard size={42} />
                <span>{isLoading ? t('loadingVideoInfo') : t('chooseOrDropVideo')}</span>
                <small>mp4 / mov / avi / mkv / webm</small>
              </button>
            )}
          </section>

          <section className="timeline-panel">
            <div className="timeline-head">
              <span>{isIndexing ? t('generatingPositionThumbnails') : t('positioningTimeline')}</span>
              <span>
                {videoInfo
                  ? `${formatTime(focusStart)} - ${formatTime(focusEnd)} · ${frames.length || 0} ${t('frameIndex')}`
                  : t('waitingImport')}
              </span>
            </div>

            <div className="timeline-stack">
              <div className="timeline-layer detail-layer">
                <div className="layer-head">
                  <span>{t('detail')}</span>
                  <strong>
                    {isDetailLoading ? t('generating') : videoInfo ? t('trackpadDetailHint') : t('waitingSelection')}
                  </strong>
                </div>

                <div className="detail-track" ref={detailTrackRef} onClick={seekFromDetailTimeline} onContextMenu={openContextMenu} onWheel={handleDetailWheel}>
                  {detailThumbnailItems.length > 0 && (
                    <div className="detail-strip">
                      {detailThumbnailItems.map((thumbnail, index) => (
                        <button
                          key={thumbnail.file}
                          type="button"
                          className={`thumbnail-button detail-thumb ${index === activeDetailThumbnail ? 'is-active' : ''}`}
                          onClick={(event) => {
                            event.stopPropagation()
                            seekToTime(thumbnail.timestamp)
                          }}
                          title={formatTime(thumbnail.timestamp)}
                        >
                          <img src={thumbnail.src} alt="" draggable={false} />
                        </button>
                      ))}
                    </div>
                  )}
                  {videoInfo && (
                    <>
                      <div
                        className="timeline-fill"
                        style={{ left: `${detailStartPercent}%`, width: `${detailRangeWidthPercent}%` }}
                      />
                      <div className="marker-line is-start" style={{ left: `${detailStartPercent}%` }} />
                      <div className="marker-line is-end" style={{ left: `${detailEndPercent}%` }} />
                      <div className="playhead detail-playhead" style={{ left: `${detailPlayheadPercent}%` }} />
                    </>
                  )}
                  {detailThumbnailItems.length === 0 && (
                    <div className="timeline-placeholder">
                      {detailMessage ? statusText(detailMessage) : videoInfo ? t('waitingDetail') : t('overviewSelectionForDetail')}
                    </div>
                  )}
                </div>

                {videoInfo && (
                  <div className="clip-action-bar">
                    <button type="button" className="clip-action-button" onClick={() => setMarker('start')}>
                      {t('setStartFrame')}
                      <kbd>{settings.shortcuts.setStartFrame.label}</kbd>
                    </button>
                    <button type="button" className="clip-action-button" onClick={() => setMarker('end')}>
                      {t('setEndFrame')}
                      <kbd>{settings.shortcuts.setEndFrame.label}</kbd>
                    </button>
                    <button type="button" className="clip-action-button is-secondary" onClick={() => void exportCurrentFrameImage()}>
                      <Download size={15} />
                      {t('exportPngShort')}
                    </button>
                    <button
                      type="button"
                      className="clip-action-button is-secondary"
                      onClick={() => void exportFrameSequence()}
                      disabled={!canExport}
                    >
                      <Download size={15} />
                      {t('exportFrameSequence')}
                    </button>
                  </div>
                )}
              </div>

              <div className="timeline-layer overview-layer">
                <div className="layer-head">
                  <span>{t('overview')}</span>
                  <strong>{t('trackpadOverviewHint')}</strong>
                </div>

                <div className="overview-track" ref={overviewTrackRef} onClick={centerFocusAt} onContextMenu={openContextMenu} onWheel={handleOverviewWheel}>
                  {overviewThumbnailItems.length > 0 && (
                    <div className="overview-strip">
                      {overviewThumbnailItems.map((thumbnail) => (
                        <button
                          key={thumbnail.file}
                          type="button"
                          className="thumbnail-button overview-thumb"
                          onClick={(event) => {
                            event.stopPropagation()
                            const length = Math.max(MIN_FOCUS_SECONDS, focusEnd - focusStart)
                            commitFocusRange(thumbnail.timestamp, thumbnail.timestamp + length)
                          }}
                          title={formatTime(thumbnail.timestamp)}
                        >
                          <img src={thumbnail.src} alt="" draggable={false} />
                        </button>
                      ))}
                    </div>
                  )}
                  {videoInfo && (
                    <>
                      <button
                        type="button"
                        className="focus-window"
                        style={{ left: `${focusPercent}%`, width: `${focusWidthPercent}%` }}
                        onPointerDown={(event) => beginFocusDrag(event, 'move')}
                        data-focus-control
                        title={t('dragSelection')}
                      >
                        <span
                          className="focus-handle is-start"
                          onPointerDown={(event) => beginFocusDrag(event, 'start')}
                          data-focus-control
                        />
                        <span
                          className="focus-handle is-end"
                          onPointerDown={(event) => beginFocusDrag(event, 'end')}
                          data-focus-control
                        />
                      </button>
                      <div className="playhead overview-playhead" style={{ left: `${overviewPlayheadPercent}%` }} />
                    </>
                  )}
                  {overviewThumbnailItems.length === 0 && (
                    <div className="timeline-placeholder">
                      {timelineMessage ? statusText(timelineMessage) : videoInfo ? t('waitingOverview') : t('dropVideoForOverview')}
                    </div>
                  )}
                </div>
              </div>
            </div>
            {videoInfo && (
              <div className="clip-status-bar">
                <span>
                  {t('setStartFrame')} <strong>{formatTime(startTime)}</strong>
                </span>
                <span>
                  {t('setEndFrame')} <strong>{formatTime(endTime)}</strong>
                </span>
                <span>
                  {t('clipLength')} <strong>{formatTime(clipDuration)}</strong>
                </span>
                <span>
                  {t('currentFormat')} <strong>{exportFormatLabel[exportFormat]}</strong>
                </span>
                {exportFormat === 'livePhoto' && (
                  <span>
                    {t('livePhotoDurationHint')}
                  </span>
                )}
              </div>
            )}
            {(timelineMessage || detailMessage) && <p className="timeline-note">{statusText(detailMessage || timelineMessage)}</p>}
          </section>

          {exportProgress && (
            <section className="progress-panel">
              <div>
                <span>{statusText(exportProgress.stage)}</span>
                <span>{exportProgress.progress}%</span>
              </div>
              <progress value={exportProgress.progress} max={100} />
            </section>
          )}

          {exportResult && (
            <section className="result-panel">
              <div className="result-paths">
                <span>{exportResult.outputPath}</span>
                {typeof exportResult.frameCount === 'number' && (
                  <span>{t('exportedFrames')}：{exportResult.frameCount}</span>
                )}
                {exportResult.videoPath && exportResult.videoPath !== exportResult.outputPath && (
                  <span>{exportResult.videoPath}</span>
                )}
                {exportResult.imagePath && exportResult.videoPath && (
                  <span className="result-hint">{t('livePhotoPairHint')}</span>
                )}
              </div>
              <button type="button" className="secondary-button" onClick={() => void openPath(exportResult.folderPath)}>
                <FolderOpen size={17} />
                {t('openFolder')}
              </button>
            </section>
          )}

          {error && (
            <section className="error-message" role="alert">
              <CircleAlert size={18} />
              <div>
                <strong>{t('operationIncomplete')}</strong>
                <span>{error}</span>
              </div>
              <button type="button" className="plain-icon-button" onClick={() => setError('')} aria-label={t('closeError')}>
                <X size={16} />
              </button>
            </section>
          )}

          {contextMenu && (
            <div className="context-menu" style={{ left: contextMenu.x, top: contextMenu.y }} onClick={(event) => event.stopPropagation()}>
              <button type="button" onClick={() => setMarkerFromContextMenu('start')}>
                {t('setStartFrame')}
                <span>I</span>
              </button>
              <button type="button" onClick={() => setMarkerFromContextMenu('end')}>
                {t('setEndFrame')}
                <span>O</span>
              </button>
              <button type="button" onClick={() => void exportCurrentFrameImage()}>
                {t('exportCurrentFrameImage')}
                <span>PNG</span>
              </button>
            </div>
          )}

          {shortcutModalOpen && (
            <div className="shortcut-modal-backdrop" onClick={() => setShortcutModalOpen(false)}>
              <section className="shortcut-modal" role="dialog" aria-modal="true" aria-label={t('keyboardShortcuts')} onClick={(event) => event.stopPropagation()}>
                <div className="shortcut-modal-head">
                  <strong>{t('keyboardShortcuts')}</strong>
                  <button type="button" className="plain-icon-button" onClick={() => setShortcutModalOpen(false)} aria-label={t('closeShortcuts')}>
                    <X size={16} />
                  </button>
                </div>
                <div className="shortcut-list">
                  {shortcutActions.map((action) => (
                    <button
                      key={action}
                      type="button"
                      className={`shortcut-row ${recordingShortcut === action ? 'is-recording' : ''}`}
                      onClick={() => setRecordingShortcut(action)}
                    >
                      <span>{t(shortcutActionLabelKey[action])}</span>
                      <kbd>{recordingShortcut === action ? t('recordingShortcut') : settings.shortcuts[action].label}</kbd>
                    </button>
                  ))}
                </div>
                <button type="button" className="secondary-button shortcut-reset-button" onClick={() => void resetShortcuts()}>
                  {t('resetShortcuts')}
                </button>
              </section>
            </div>
          )}
        </section>
      </div>
    </main>
  )
}

function formatTime(value: number) {
  if (!Number.isFinite(value)) {
    return '00:00.000'
  }
  const minutes = Math.floor(value / 60)
  const seconds = value % 60
  return `${minutes.toString().padStart(2, '0')}:${seconds.toFixed(3).padStart(6, '0')}`
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

function normalizeShortcuts(shortcuts: KeyboardShortcuts | null | undefined): KeyboardShortcuts {
  return {
    ...defaultShortcuts,
    ...(shortcuts ?? {}),
  }
}

function shortcutFromEvent(event: KeyboardEvent): ShortcutBinding | null {
  if (['Control', 'Meta', 'Alt', 'Shift'].includes(event.key)) {
    return null
  }

  return {
    key: event.key,
    code: event.code,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    altKey: event.altKey,
    shiftKey: event.shiftKey,
    label: formatShortcutLabel(event),
  }
}

function shortcutMatches(shortcut: ShortcutBinding, event: KeyboardEvent) {
  if (shortcut.code !== event.code) {
    return false
  }

  const systemModifierMatches =
    shortcut.ctrlKey && shortcut.metaKey
      ? event.ctrlKey || event.metaKey
      : shortcut.ctrlKey === event.ctrlKey && shortcut.metaKey === event.metaKey

  return systemModifierMatches && shortcut.altKey === event.altKey && shortcut.shiftKey === event.shiftKey
}

function formatShortcutLabel(event: KeyboardEvent) {
  const parts: string[] = []
  if (event.metaKey) {
    parts.push('Cmd')
  }
  if (event.ctrlKey) {
    parts.push('Ctrl')
  }
  if (event.altKey) {
    parts.push('Alt')
  }
  if (event.shiftKey) {
    parts.push('Shift')
  }
  parts.push(formatKeyLabel(event.key, event.code))
  return parts.join(' + ')
}

function formatKeyLabel(key: string, code: string) {
  if (code === 'Space') {
    return 'Space'
  }
  if (key === 'ArrowLeft') {
    return 'Left'
  }
  if (key === 'ArrowRight') {
    return 'Right'
  }
  if (key === 'ArrowUp') {
    return 'Up'
  }
  if (key === 'ArrowDown') {
    return 'Down'
  }
  if (key.length === 1) {
    return key.toUpperCase()
  }
  return key
}

function clampFocusRange(start: number, end: number, duration: number): [number, number] {
  const safeDuration = Math.max(duration, MIN_FOCUS_SECONDS)
  const rangeLength = clamp(end - start, MIN_FOCUS_SECONDS, safeDuration)
  let nextStart = clamp(start, 0, Math.max(0, safeDuration - rangeLength))
  let nextEnd = nextStart + rangeLength

  if (nextEnd > duration) {
    nextEnd = duration
    nextStart = Math.max(0, nextEnd - rangeLength)
  }

  return [nextStart, nextEnd]
}

function focusCenterTime(start: number, end: number) {
  return start + (end - start) / 2
}

function buildThumbnailItems(thumbnails: ThumbnailResult | null) {
  if (!thumbnails) {
    return []
  }

  return thumbnails.files.map((file, index) => ({
    file,
    src: runningInTauri ? convertFileSrc(file) : file,
    timestamp: thumbnails.startTime + index * thumbnails.intervalSeconds,
  }))
}

function nearestFrameIndex(frames: FramePoint[], timestamp: number) {
  if (frames.length === 0) {
    return 0
  }

  let low = 0
  let high = frames.length - 1

  while (low < high) {
    const mid = Math.floor((low + high) / 2)
    if (frames[mid].timestamp < timestamp) {
      low = mid + 1
    } else {
      high = mid
    }
  }

  const previous = Math.max(0, low - 1)
  const previousDelta = Math.abs(frames[previous].timestamp - timestamp)
  const currentDelta = Math.abs(frames[low].timestamp - timestamp)
  return previousDelta <= currentDelta ? previous : low
}

function adjacentFrameIndex(frames: FramePoint[], timestamp: number, direction: -1 | 1) {
  if (frames.length === 0) {
    return 0
  }

  const nearest = nearestFrameIndex(frames, timestamp)
  const previousGap = nearest > 0 ? frames[nearest].timestamp - frames[nearest - 1].timestamp : Number.POSITIVE_INFINITY
  const nextGap = nearest + 1 < frames.length ? frames[nearest + 1].timestamp - frames[nearest].timestamp : Number.POSITIVE_INFINITY
  const localGap = Math.min(previousGap, nextGap)
  const tolerance = Number.isFinite(localGap) ? Math.max(0.000001, localGap / 4) : 0.001
  const isOnNearestFrame = Math.abs(frames[nearest].timestamp - timestamp) <= tolerance

  let target = nearest
  if (isOnNearestFrame) {
    target += direction
  } else if (direction > 0 && frames[nearest].timestamp < timestamp) {
    target += 1
  } else if (direction < 0 && frames[nearest].timestamp > timestamp) {
    target -= 1
  }

  return Math.max(0, Math.min(frames.length - 1, target))
}

export default App
