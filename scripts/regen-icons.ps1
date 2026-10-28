# Zapdude icon consistency pass (green squircle + BLACK bolt) — fixes the
# stale gray-teal/white-bolt icon.exe embed:
#   A) rebuild icons/icon.ico byte-for-byte from the APPROVED pngs
#   B) re-render icons/icon.png (512) + Square*Logo + StoreLogo in style
#   C) verification sheet for visual comparison
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$outDir = "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\icons"

# ---------- B) vector renderer (recipe from the approved generator) ----------
function New-ZapdudeIcon {
    param([int]$Size)
    $bmp = New-Object System.Drawing.Bitmap($Size, $Size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    $g.Clear([System.Drawing.Color]::Transparent)

    $r = [float]($Size * 0.23)
    $rect = New-Object System.Drawing.RectangleF(0, 0, [float]$Size, [float]$Size)
    $round = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $round.AddArc(0, 0, $d, $d, 180, 90)
    $round.AddArc(($Size - $d), 0, $d, $d, 270, 90)
    $round.AddArc(($Size - $d), ($Size - $d), $d, $d, 0, 90)
    $round.AddArc(0, ($Size - $d), $d, $d, 90, 90)
    $round.CloseFigure()

    # Vertical green gradient (same stops as the rail/drawer brand tile)
    $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect,
        [System.Drawing.Color]::FromArgb(255,52,232,119), [System.Drawing.Color]::FromArgb(255,15,158,108), 90)
    $g.FillPath($grad, $round)

    # Diagonal corner gloss like the approved tile (soft alpha, sharp chord)
    $gloss = New-Object System.Drawing.Drawing2D.GraphicsPath
    $gloss.AddPolygon([System.Drawing.PointF[]]@(
        (New-Object System.Drawing.PointF(0.0, 0.0)),
        (New-Object System.Drawing.PointF([float]($Size * 0.98), 0.0)),
        (New-Object System.Drawing.PointF([float]($Size * 0.42), [float]($Size * 0.62))),
        (New-Object System.Drawing.PointF(0.0, [float]($Size * 0.30)))
    ))
    $glossBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(52,255,255,255))
    $g.SetClip($round)
    $g.FillPath($glossBrush, $gloss)
    $g.ResetClip()

    # Black bolt — rail.html #i-bolt path converted to GDI+
    $scale = ($Size * 0.60) / 24.0
    $ox = [float](($Size - 24.0 * $scale) / 2.0)
    $oy = [float](($Size - 24.0 * $scale) / 2.0)
    $black = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,4,35,26))

    function AddBolt([System.Drawing.Drawing2D.GraphicsPath]$path, [System.Drawing.Graphics]$gfx, [double]$sc, [double]$x0, [double]$y0) {
        $pt = { param($x, $y) New-Object System.Drawing.PointF(([float]($x0 + $x * $sc)), ([float]($y0 + $y * $sc))) }.GetNewClosure()
        $path.AddBezier((& $pt 13.2 2.2), (& $pt 13.6 1.7), (& $pt 14.4 2.1), (& $pt 14.3 2.7))
        $path.AddLine((& $pt 14.3 2.7), (& $pt 13.2 9.3))
        $path.AddLine((& $pt 13.2 9.3), (& $pt 18.3 9.3))
        $path.AddBezier((& $pt 18.3 9.3), (& $pt 18.9 9.3), (& $pt 19.2 10.0), (& $pt 18.8 10.4))
        $path.AddLine((& $pt 18.8 10.4), (& $pt 10.8 21.8))
        $path.AddBezier((& $pt 10.8 21.8), (& $pt 10.4 22.3), (& $pt 9.6 21.9), (& $pt 9.7 21.3))
        $path.AddLine((& $pt 9.7 21.3), (& $pt 10.8 14.7))
        $path.AddLine((& $pt 10.8 14.7), (& $pt 5.7 14.7))
        $path.AddBezier((& $pt 5.7 14.7), (& $pt 5.1 14.7), (& $pt 4.8 14.0), (& $pt 5.2 13.6))
        $path.CloseFigure()
        $gfx.FillPath($black, $path)
    }
    $boltPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    AddBolt $boltPath $g $scale $ox $oy
    $round.Dispose(); $grad.Dispose(); $black.Dispose(); $boltPath.Dispose()
    $g.Dispose()
    return $bmp
}

# ---------- A) icon.ico from the APPROVED png frames ----------
$icoSizes = 16,24,32,48,64,128,256
$frames = @{}
foreach ($s in $icoSizes) {
    $file = Join-Path $outDir ("$s x$s.png" -replace ' ', '')
    $frames[$s] = [System.IO.File]::ReadAllBytes($file)
}
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$icoSizes.Count)
$offset = 6 + 16 * $icoSizes.Count
foreach ($s in $icoSizes) {
    $data = $frames[$s]
    $bw.Write([byte]($s -band 0xFF)); $bw.Write([byte]($s -band 0xFF))
    $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([uint16]1); $bw.Write([uint16]32)
    $bw.Write([uint32]$data.Length); $bw.Write([uint32]$offset)
    $offset += [uint32]$data.Length
}
foreach ($s in $icoSizes) { $bw.Write($frames[$s]) }
$bw.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $outDir "icon.ico"), $ms.ToArray())
$bw.Dispose(); $ms.Dispose()
"ICO rebuilt: 7 frames ($($icoSizes -join ', ')) from approved PNG bytes"

# ---------- B2) render the remaining assets ----------
$targets = @{
    "icon.png" = 512
    "Square30x30Logo.png" = 30; "Square44x44Logo.png" = 44; "Square71x71Logo.png" = 71
    "Square89x89Logo.png" = 89; "Square107x107Logo.png" = 107; "Square142x142Logo.png" = 142
    "Square150x150Logo.png" = 150; "Square284x284Logo.png" = 284; "Square310x310Logo.png" = 310
    "StoreLogo.png" = 50
}
$render512 = $null
foreach ($name in $targets.Keys) {
    $size = $targets[$name]
    if ($size -eq 512) { if (-not $render512) { $render512 = New-ZapdudeIcon 512 } ; $bmp = $render512 }
    else { $bmp = New-ZapdudeIcon $size }
    $bmp.Save((Join-Path $outDir $name), [System.Drawing.Imaging.ImageFormat]::Png)
    "rendered $name ($size px)"
}
if ($render512) { $render512.Dispose() }

# ---------- C) verification sheet: new ico-256 vs approved 256 vs old-style ----------
$sheet = [System.Drawing.Bitmap]::new(3*280, 320)
$g = [System.Drawing.Graphics]::FromImage($sheet)
$g.Clear([System.Drawing.Color]::FromArgb(255,58,58,58))
$images = @(
    @{ p = "C:\Users\PC\AppData\Local\Temp\opencode\ico-frames\frame-5-256.png"; t = "OLD icon.ico 256" },
    @{ p = (Join-Path $outDir "256x256.png"); t = "approved 256 png" },
    @{ p = (Join-Path $outDir "icon.png"); t = "new icon.png 512" }
)
for ($i = 0; $i -lt 3; $i++) {
    $img = [System.Drawing.Image]::FromFile($images[$i].p)
    $g.DrawImage($img, ($i*280+21), 16, 256, 256)
    $font = [System.Drawing.Font]::new("Consolas", 11)
    $g.DrawString($images[$i].t, $font, [System.Drawing.Brushes]::White, ($i*280+21), 278)
    $font.Dispose(); $img.Dispose()
}
$g.Dispose()
$sheet.Save("C:\Users\PC\AppData\Local\Temp\opencode\icon-sheet\compare.png", [System.Drawing.Imaging.ImageFormat]::Png)
$sheet.Dispose()
"SHEET OK"
