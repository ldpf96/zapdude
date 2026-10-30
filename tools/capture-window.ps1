<#
.SYNOPSIS
  PrintWindow capture of every visible top-level window of the zapdude
  process (PW_RENDERFULLCONTENT so WebView2 surfaces compose). Used to
  inspect call prompts / rail states without screen-cast dependencies.
#>
param([string]$OutDir = "C:\Users\PC\AppData\Local\Temp\opencode\winframes", [switch]$NoWipe)

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WinCap {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr data);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc cb, IntPtr data);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
Add-Type -AssemblyName System.Drawing

[WinCap]::SetProcessDPIAware() | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if (-not $NoWipe) {
    Get-ChildItem $OutDir -Filter *.png -ErrorAction SilentlyContinue | Remove-Item -Force
}

$procs = Get-Process zapdude -ErrorAction SilentlyContinue
if (-not $procs) { "NO_ZAPDUDE_PROCESS"; exit 1 }
$pids = @($procs.Id | ForEach-Object { [int]$_ })

$hits = New-Object System.Collections.ArrayList
$cb = [WinCap+EnumWindowsProc]{
  param($h, $l)
  [uint32]$pid2 = 0
  [WinCap]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
  if ($pids -contains [int]$pid2 -and [WinCap]::IsWindowVisible($h)) {
    $len = [WinCap]::GetWindowTextLength($h)
    $sb = New-Object System.Text.StringBuilder ($len + 4)
    [WinCap]::GetWindowText($h, $sb, $sb.Capacity) | Out-Null
    [void]$hits.Add([pscustomobject]@{ Hwnd = $h; Title = $sb.ToString() })
  }
  return $true
}
[WinCap]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null

$stamp = Get-Date -Format "HHmmss"
$i = 0
$c = 0
foreach ($hit in $hits) {
  $rect = New-Object WinCap+RECT
  [WinCap]::GetWindowRect($hit.Hwnd, [ref]$rect) | Out-Null
  $w = $rect.Right - $rect.Left
  $h2 = $rect.Bottom - $rect.Top
  if ($w -le 0 -or $h2 -le 0) { continue }
  $bmp = New-Object System.Drawing.Bitmap $w, $h2
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [WinCap]::PrintWindow($hit.Hwnd, $hdc, 2) | Out-Null   # 2 = PW_RENDERFULLCONTENT
  $g.ReleaseHdc($hdc)
  $g.Dispose()
  $name = "win_${stamp}_$i.png"
  $bmp.Save("$OutDir\$name", [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "captured hwnd=$($hit.Hwnd) title='$($hit.Title)' ${w}x${h2} -> $name"
  $i++
  # The account webviews live as CHILD windows inside the main one; a
  # parked account is not the visible one, but PrintWindow can still
  # fetch its compositor surface. Save every big child (>250x180).
  $kids = New-Object System.Collections.ArrayList
  $kidCb = [WinCap+EnumWindowsProc]{
    param($ch, $l)
    $kr = New-Object WinCap+RECT
    [WinCap]::GetWindowRect($ch, [ref]$kr) | Out-Null
    $kw = $kr.Right - $kr.Left
    $kh = $kr.Bottom - $kr.Top
    if ($kw -gt 250 -and $kh -gt 180) { [void]$kids.Add([pscustomobject]@{ Hwnd = $ch; W = $kw; H = $kh }) }
    return $true
  }
  [WinCap]::EnumChildWindows($hit.Hwnd, $kidCb, [IntPtr]::Zero) | Out-Null
  foreach ($kid in $kids) {
    if ($c -ge 8) { break }
    try {
      $kbmp = New-Object System.Drawing.Bitmap $kid.W, $kid.H
      $kg = [System.Drawing.Graphics]::FromImage($kbmp)
      $khdc = $kg.GetHdc()
      [WinCap]::PrintWindow($kid.Hwnd, $khdc, 2) | Out-Null
      $kg.ReleaseHdc($khdc)
      $kg.Dispose()
      $kname = "child_${stamp}_$c`_$($kid.W)x$($kid.H).png"
      $kbmp.Save("$OutDir\$kname", [System.Drawing.Imaging.ImageFormat]::Png)
      $kbmp.Dispose()
      "captured child hwnd=$($kid.Hwnd) $($kid.W)x$($kid.H) -> $kname"
      $c++
    } catch {
      # a child that disappears mid-walk is not worth crashing over
    }
  }
}
"CAPTURED=$i CHILDREN=$c"
