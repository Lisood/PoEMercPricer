param(
    [string]$Title = "PoEMercPricer",
    [string]$Out = "debug/live-overlay.png",
    [int]$WaitSeconds = 20,
    # Trim an existing capture in place instead of taking a new one.
    [string]$TrimOnly
)

if (-not ('NativeWin' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class NativeWin {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out RECT rect, int size);
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@
}

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

# The visible frame still takes in a 1 px border and the rounded-corner cutouts, and both show the
# desktop behind the window. Drop the border and repaint each corner from just inside it.
function Remove-DesktopEdge($src) {
    if ($src.Width -lt 16 -or $src.Height -lt 16) { return $src.Clone() }
    $w = $src.Width - 2
    $h = $src.Height - 2
    $dst = $src.Clone([System.Drawing.Rectangle]::new(1, 1, $w, $h), [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    for ($i = 0; $i -lt 4; $i++) {
        for ($j = 0; $j -lt 4; $j++) {
            $dst.SetPixel($i, $j, $dst.GetPixel(6, 6))
            $dst.SetPixel($w - 1 - $i, $j, $dst.GetPixel($w - 7, 6))
            $dst.SetPixel($i, $h - 1 - $j, $dst.GetPixel(6, $h - 7))
            $dst.SetPixel($w - 1 - $i, $h - 1 - $j, $dst.GetPixel($w - 7, $h - 7))
        }
    }
    $dst
}

if ($TrimOnly) {
    $path = (Resolve-Path -LiteralPath $TrimOnly).Path
    $src = [System.Drawing.Bitmap]::FromFile($path)
    $trimmed = Remove-DesktopEdge $src
    $src.Dispose()   # FromFile holds the file open until then
    $trimmed.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Output "TRIMMED=$TrimOnly $($trimmed.Width)x$($trimmed.Height)"
    $trimmed.Dispose()
    exit 0
}

$proc = $null
for ($i = 0; $i -lt $WaitSeconds; $i++) {
    $proc = Get-Process | Where-Object { $_.ProcessName -eq 'poemercpricer' -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $proc) {
        $proc = Get-Process | Where-Object { $_.MainWindowTitle -eq $Title -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    }
    if ($proc) { break }
    Start-Sleep -Seconds 1
}
if (-not $proc) {
    Write-Output "WINDOW_NOT_FOUND"
    Get-Process poemercpricer -ErrorAction SilentlyContinue | Format-Table Id, ProcessName, MainWindowTitle | Out-String
    exit 1
}

$hwnd = $proc.MainWindowHandle
[NativeWin]::ShowWindow($hwnd, 9) | Out-Null
[NativeWin]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 400

$rect = New-Object NativeWin+RECT
[NativeWin]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
# GetWindowRect includes the invisible resize border, which captures a strip of whatever is behind the
# window (other apps' text). DWMWA_EXTENDED_FRAME_BOUNDS (9) is the visible frame only.
$visible = New-Object NativeWin+RECT
if ([NativeWin]::DwmGetWindowAttribute($hwnd, 9, [ref]$visible, 16) -eq 0) { $rect = $visible }
$w = [Math]::Max(1, $rect.Right - $rect.Left)
$h = [Math]::Max(1, $rect.Bottom - $rect.Top)
Write-Output "HWND=$hwnd RECT=$($rect.Left),$($rect.Top) ${w}x${h} PID=$($proc.Id) TITLE='$($proc.MainWindowTitle)'"

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
$outPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Out)
$dir = Split-Path -Parent $outPath
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$g.Dispose()
$trimmed = Remove-DesktopEdge $bmp
$bmp.Dispose()
$trimmed.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
$trimmed.Dispose()
Write-Output "SAVED=$Out"
