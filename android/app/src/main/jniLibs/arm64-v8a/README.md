# arm64-v8a FFmpeg

Place the self-built Android arm64-v8a FFmpeg PIE executable here:

```text
libffmpeg.so
```

The filename uses the `.so` convention so Android packages it under `nativeLibraryDir`, but the file is executed by `ProcessBuilder`.
