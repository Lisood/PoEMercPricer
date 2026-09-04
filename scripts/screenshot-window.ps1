param(
    [string]$Title = "PoEMercPricer",
    [string]$Out = "debug/live-overlay.png",
    [int]$WaitSeconds = 20
)

if (-not ('NativeWin' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class NativeWin {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@
}

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

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
$w = [Math]::Max(1, $rect.Right - $rect.Left)
$h = [Math]::Max(1, $rect.Bottom - $rect.Top)
Write-Output "HWND=$hwnd RECT=$($rect.Left),$($rect.Top) ${w}x${h} PID=$($proc.Id) TITLE='$($proc.MainWindowTitle)'"

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
$outPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Out)
$dir = Split-Path -Parent $outPath
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Output "SAVED=$Out"
