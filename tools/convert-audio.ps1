# Convert Soulscale's WAV masters into web formats for the Discord client (Windows).
#
#   powershell -ExecutionPolicy Bypass -File tools\convert-audio.ps1 -Source D:\audio-assets
#   powershell -ExecutionPolicy Bypass -File tools\convert-audio.ps1 -Source D:\audio-assets -Out client\public\audio
#
# -Source holds music\*.wav and sfx\*.wav. Output (default client\public\audio):
#   <name>.ogg  Ogg Opus (Chrome / Discord desktop / Android; gapless loops)
#   <name>.m4a  AAC fallback (Safari / iOS Discord WebView)
#   index.json  every asset with exact duration and sample count (for loop points)
# The hand-edited event->sound mapping (manifest.json) is never touched.
# Needs ffmpeg + ffprobe on PATH:  winget install Gyan.FFmpeg
param(
    [Parameter(Mandatory = $true)][string]$Source,
    [string]$Out = "client\public\audio"
)
$ErrorActionPreference = "Stop"
foreach ($tool in "ffmpeg", "ffprobe") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool not found on PATH. Install with: winget install Gyan.FFmpeg (then open a new terminal)"
    }
}

function Probe([string]$file, [string]$entry) {
    (& ffprobe -v error -select_streams a:0 -show_entries "stream=$entry" -of default=nw=1:nk=1 $file).Trim()
}

$entries = New-Object System.Collections.Generic.List[string]
foreach ($kind in "music", "sfx") {
    $dir = Join-Path $Source $kind
    if (-not (Test-Path $dir)) { continue }
    $dest = Join-Path $Out $kind
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    if ($kind -eq "music") { $opus = "128k"; $aac = "160k" } else { $opus = "96k"; $aac = "128k" }
    foreach ($wav in Get-ChildItem -Path $dir -Filter *.wav | Sort-Object Name) {
        $name = [IO.Path]::GetFileNameWithoutExtension($wav.Name)
        if ($name -notmatch '^[A-Za-z0-9._-]+$') { Write-Warning "skip (rename without spaces/symbols): $($wav.FullName)"; continue }
        & ffmpeg -hide_banner -loglevel error -y -i $wav.FullName -map_metadata -1 -vn `
            -c:a libopus -b:a $opus -vbr on -application audio -ar 48000 (Join-Path $dest "$name.ogg")
        if ($LASTEXITCODE -ne 0) { throw "ffmpeg (opus) failed on $($wav.Name)" }
        & ffmpeg -hide_banner -loglevel error -y -i $wav.FullName -map_metadata -1 -vn `
            -c:a aac -b:a $aac -ar 48000 -movflags +faststart (Join-Path $dest "$name.m4a")
        if ($LASTEXITCODE -ne 0) { throw "ffmpeg (aac) failed on $($wav.Name)" }
        $rate = Probe $wav.FullName "sample_rate"
        $chans = Probe $wav.FullName "channels"
        $samples = Probe $wav.FullName "duration_ts"
        $dur = (& ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 $wav.FullName).Trim()
        $entries.Add("{`"id`":`"$kind/$name`",`"kind`":`"$kind`",`"duration`":$dur,`"rate`":$rate,`"samples`":$samples,`"channels`":$chans}")
        Write-Host "  $kind/$name  ($dur s)"
    }
}
$json = "{`"version`":1,`"assets`":[`n" + ($entries -join ",`n") + "`n]}`n"
[IO.File]::WriteAllText((Join-Path (Resolve-Path $Out) "index.json"), $json, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "wrote $($entries.Count) assets to $Out (index.json updated; manifest.json untouched)"
