# Discord Video Preview

Makes the 10-second clip Discord shows when someone hovers over Soulscale in the Activity menu
(and on its upsell card): 640x360, H.264 MP4, under 0.5 MB, no sound.

**What's in it:** the game in a 4:3 window, framed by original artwork: a praying angel in gold light
on the left, a demon in ember light reaching toward the board on the right (`art.html`). Four cuts
from one real game, crossfaded:

1. the title and sigil ring
2. browsing the hand, picking Hope, Chambers lighting up
3. cards flying into Time, a 5 vs 7 clash, the Soul being pulled
4. the Soul dropping to −12: "Hell wins"

## Remaking it (after the game's look changes)

Needs Python with `playwright` and `Pillow`, Chromium (`py -m playwright install chromium`), and
ffmpeg with libx264. Run the server with browser test logins on (`tools\run.ps1 -NoBrowser`), then:

```powershell
py tools\preview\record.py --out preview-raw          # plays a whole game at a human pace (2-4 min)
py tools\preview\make_preview.py sheet preview-raw 150 190 --out sheet.png   # find the good moments
```

Every recording is a different game, so look at the contact sheets and set new times (and crops,
in the 960x720 recording) in `cuts.json`. Then:

```powershell
py tools\preview\make_preview.py build preview-raw --out soulscale-preview.mp4
```

It fits the cuts to exactly 10.0 s and lowers the bitrate until the file is under 480,000 bytes.

Upload it in the Developer Portal where your Activity asks for its **Video Preview**.
