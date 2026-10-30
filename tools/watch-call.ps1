<#
.SYNOPSIS
  Call-verification watcher: a fast CopyFromScreen beat (catches the OS
  toast, which PrintWindow cannot see) plus a few PrintWindow shots of
  the app window while an incoming call rings. All frames accumulate in
  one folder with second-resolution stamps in the names.
#>
param(
    [int]$Seconds = 340,
    [string]$OutDir = "C:\Users\PC\AppData\Local\Temp\opencode\callframes3"
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WinCapW {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr data);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

[WinCapW]::SetProcessDPIAware() | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$script = "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\tools\capture-window.ps1"
$vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
$printAt = @(8, 18, 28, 40, 52, 64, 80)
$sw = New-Object System.Diagnostics.Stopwatch
$sw.Start()
$beat = 0
while ($sw.Elapsed.TotalSeconds -lt $Seconds) {
    $beat++
    try {
        $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($vs.X, $vs.Y, 0, 0, $vs.Size)
        $sec = '{0:d3}' -f [int]$sw.Elapsed.TotalSeconds
        $bmp.Save("$OutDir\screen_$sec.png", [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose()
        $bmp.Dispose()
    } catch {
        # a lost beat is fine; keep the loop alive
    }
    $t = [int]$sw.Elapsed.TotalSeconds
    if ($printAt -contains $t) {
        & $script -OutDir $OutDir -NoWipe | Out-Null   # win_* PrintWindow shots
    }
    Start-Sleep -Milliseconds 800
}
$screenFiles = (Get-ChildItem $OutDir -Filter screen_*.png -ErrorAction SilentlyContinue | Measure-Object).Count
$winFiles = (Get-ChildItem $OutDir -Filter win_*.png -ErrorAction SilentlyContinue | Measure-Object).Count
"SCREEN_FRAMES=$screenFiles PRINT_FRAMES=$winFiles"
