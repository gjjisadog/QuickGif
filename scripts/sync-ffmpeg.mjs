import { copyFile, chmod, mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import ffmpegPath from 'ffmpeg-static'
import ffprobe from 'ffprobe-static'

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const targetDir = join(repoRoot, 'src-tauri', 'binaries')
const suffix = process.platform === 'win32' ? '.exe' : ''

await mkdir(targetDir, { recursive: true })

await copyTool(ffmpegPath, `ffmpeg${suffix}`)
await copyTool(ffprobe.path, `ffprobe${suffix}`)

async function copyTool(source, name) {
  const target = join(targetDir, name)
  await copyFile(source, target)
  if (process.platform !== 'win32') {
    await chmod(target, 0o755)
  }
  console.log(`Prepared ${name}`)
}
