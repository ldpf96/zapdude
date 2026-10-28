# Dump every frame of a .ico to PNG/bmp files so we can visually inspect it.
param(
  [string]$IcoPath = "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\icons\icon.ico",
  [string]$OutDir = "C:\Users\PC\AppData\Local\Temp\opencode\ico-frames"
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$bits = [System.IO.File]::ReadAllBytes($IcoPath)
Write-Output ("Total bytes: " + $bits.Length)
$count = [BitConverter]::ToUInt16($bits, 4)
Write-Output ("Entries: " + $count)
for ($i = 0; $i -lt $count; $i++) {
  $o = 6 + 16 * $i
  $w = $bits[$o]; if ($w -eq 0) { $w = 256 }
  $h = $bits[$o+1]; if ($h -eq 0) { $h = 256 }
  $bpp = [BitConverter]::ToUInt16($bits, $o+6)
  $size = [BitConverter]::ToUInt32($bits, $o+8)
  $off = [BitConverter]::ToUInt32($bits, $o+12)
  $frame = New-Object byte[] $size
  [Array]::Copy($bits, $off, $frame, 0, $size)
  $isPng = ($size -gt 8 -and $frame[1] -eq 0x50 -and $frame[2] -eq 0x4E -and $frame[3] -eq 0x47)
  Write-Output ("frame $i : ${w}x${h} bpp=$bpp size=$size png=$isPng")
  if ($isPng) {
    [System.IO.File]::WriteAllBytes((Join-Path $OutDir ("frame-$i-" + $w + ".png")), $frame)
  } else {
    # BMP frame: blob = BITMAPINFOHEADER(+palette) + XOR bitmap + AND mask.
    # Rebuild a .bmp file by prepending a 14-byte BITMAPFILEHEADER.
    $bmiSize = [BitConverter]::ToUInt32($frame, 0)
    $clrUsed = [BitConverter]::ToUInt32($frame, 32)
    if ($clrUsed -eq 0 -and $bpp -le 8) {
      # infer full palette for low bpp
      $pal = [Math]::Pow(2, $bpp)
    } else { $pal = $clrUsed }
    $xorSize = $size - $bmiSize - 4 * $pal
    $fileHeader = New-Object byte[] 14
    $fileHeader[0] = 0x42; $fileHeader[1] = 0x4D
    $fileLen = 14 + $size - (4 * 0)  # AND mask included in blob already; header offset trick below
    # bfOffBits = 14 + bmiSize(+palette). Simplest: write header with offbits = 14 + $bmiSize + 4*$pal... but blob already contains palette; keep offbits = 14 + ($size - $xorSize)
    $offbits = 14 + ($size - $xorSize)
    [Array]::Copy([BitConverter]::GetBytes([int]($fileLen)), 0, $fileHeader, 2, 4)
    [Array]::Copy([BitConverter]::GetBytes([int]$offbits), 0, $fileHeader, 10, 4)
    $bmp = New-Object byte[] (14 + $size)
    [Array]::Copy($fileHeader, 0, $bmp, 0, 14)
    [Array]::Copy($frame, 0, $bmp, 14, $size)
    [System.IO.File]::WriteAllBytes((Join-Path $OutDir ("frame-$i-" + $w + ".bmp")), $bmp)
    # convert to PNG for easy viewing
    $img = [System.Drawing.Bitmap]::new((Join-Path $OutDir ("frame-$i-" + $w + ".bmp")))
    if ($w -le 64) {
      $big = [System.Drawing.Bitmap]::new(256, 256)
      $g = [System.Drawing.Graphics]::FromImage($big)
      $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
      $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
      $g.DrawImage($img, 0, 0, 256, 256)
      $g.Dispose()
      $big.Save((Join-Path $OutDir ("frame-$i-" + $w + "-big.png")), [System.Drawing.Imaging.ImageFormat]::Png)
      $big.Dispose()
    } else {
      $img.Save((Join-Path $OutDir ("frame-$i-" + $w + ".png")), [System.Drawing.Imaging.ImageFormat]::Png)
    }
    $img.Dispose()
  }
}
Get-ChildItem $OutDir | Select-Object Name, Length | Format-Table -AutoSize
