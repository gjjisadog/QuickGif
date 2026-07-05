package com.quickgif.app.ffmpeg

import com.quickgif.app.data.GifExportConfig
import com.quickgif.app.data.GifFrameRateOption
import com.quickgif.app.data.GifQualityOption
import com.quickgif.app.data.GifWidthOption
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class FFmpegCommandBuilderTest {
    @Test
    fun buildUsesTwoStepPaletteFlowAndLanczosScale() {
        val config = GifExportConfig(
            inputFile = File("/tmp/中文 path/input video.mp4"),
            outputFile = File("/tmp/out gif.gif"),
            paletteFile = File("/tmp/palette file.png"),
            startMs = 1234,
            endMs = 6234,
            width = GifWidthOption.W480,
            frameRate = GifFrameRateOption.Fps15,
            quality = GifQualityOption.Standard
        )

        val commands = FFmpegCommandBuilder().build(config)

        assertTrue(commands.paletteCommand.contains("/tmp/中文 path/input video.mp4"))
        assertTrue(commands.paletteCommand.contains("/tmp/palette file.png"))
        assertTrue(commands.paletteCommand.contains("fps=15,scale=480:-1:flags=lanczos,palettegen=max_colors=192"))

        assertTrue(commands.gifCommand.contains("-lavfi"))
        assertTrue(commands.gifCommand.contains("fps=15,scale=480:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=sierra2_4a"))
        assertTrue(commands.gifCommand.contains("-loop"))
        assertTrue(commands.gifCommand.contains("0"))
        assertTrue(commands.gifCommand.contains("/tmp/out gif.gif"))
    }

    @Test
    fun buildKeepsEachPathAsOneArgument() {
        val config = GifExportConfig(
            inputFile = File("/tmp/space name/input video.mov"),
            outputFile = File("/tmp/space name/out.gif"),
            paletteFile = File("/tmp/space name/palette.png"),
            startMs = 0,
            endMs = 1000,
            width = GifWidthOption.Original,
            frameRate = GifFrameRateOption.Original,
            quality = GifQualityOption.High
        )

        val commands = FFmpegCommandBuilder().build(config)

        assertEquals("/tmp/space name/input video.mov", commands.paletteCommand[6])
        assertEquals("/tmp/space name/palette.png", commands.paletteCommand.last())
        assertTrue(commands.gifCommand.contains("scale=iw:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=sierra2_4a"))
    }
}
