#Requires -PSEdition Desktop
# WinRT projection via System.Runtime.WindowsRuntime only exists in Windows PowerShell 5.1.
param([string]$Path = "samples\manyshot_alara.png")
$ErrorActionPreference = 'Stop'
$full = (Resolve-Path $Path).Path
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
$null = [Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics,ContentType=WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine,Windows.Media,ContentType=WindowsRuntime]
function Await($WinRtTask, $resultType) {
  $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.IsGenericMethod
  } | Select-Object -First 1)
  $net = $asTaskGeneric.MakeGenericMethod($resultType).Invoke($null, @($WinRtTask))
  $net.Wait()
  $net.Result
}
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($full)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if (-not $engine) { throw "No Windows OCR language is installed; add English with optical character recognition under Settings > Time & Language." }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
Write-Output "LANG=$($engine.RecognizerLanguage.LanguageTag)"
Write-Output "TEXT<<"
Write-Output $result.Text
Write-Output ">>"
Write-Output "LINES=$($result.Lines.Count)"
foreach ($line in $result.Lines) {
  Write-Output ("LINE: " + $line.Text)
}
