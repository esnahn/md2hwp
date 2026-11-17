param([Parameter(Mandatory=$true)][string]$Directory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$null = New-Item -ItemType Directory -Force -Path $Directory
$reference = New-Object Drawing.Bitmap 32,32
$referenceGraphics = [Drawing.Graphics]::FromImage($reference)
$dc = $referenceGraphics.GetHdc()
try {
 foreach ($variant in @('EmfOnly','EmfPlusDual','EmfPlusOnly')) {
  $path = Join-Path $Directory ($variant+'.emf')
  $type = [Drawing.Imaging.EmfType]::$variant
  $frame = New-Object Drawing.RectangleF 0,0,142,80
  $meta = [Drawing.Imaging.Metafile]::new($path,$dc,$frame,[Drawing.Imaging.MetafileFrameUnit]::Millimeter,$type)
  $g = [Drawing.Graphics]::FromImage($meta)
  $owned = @()
  try {
   $g.PageUnit = [Drawing.GraphicsUnit]::Millimeter
   $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
   $g.TextRenderingHint = [Drawing.Text.TextRenderingHint]::AntiAliasGridFit
   $black = [Drawing.Pen]::new([Drawing.Color]::Black,0.2); $owned += $black
   $blue = [Drawing.Pen]::new([Drawing.Color]::Blue,0.6); $owned += $blue
   $dash = [Drawing.Pen]::new([Drawing.Color]::DarkGreen,0.4); $owned += $dash
   $dash.DashStyle = [Drawing.Drawing2D.DashStyle]::Dash
   $font = [Drawing.Font]::new('Malgun Gothic',12,[Drawing.FontStyle]::Regular,[Drawing.GraphicsUnit]::Point); $owned += $font
   if ($font.Name -ne 'Malgun Gothic') { throw 'Fixture font fallback' }
   $g.FillRectangle([Drawing.Brushes]::White,0,0,142,80)
   $g.DrawRectangle($black,1,1,140,78)
   $g.DrawLine($black,5,7,135,7); $g.DrawLine($blue,5,11,135,11); $g.DrawLine($dash,5,15,135,15)
   $g.DrawEllipse($black,5,20,18,10); $g.DrawRectangle($blue,27,20,18,10)
   $korean = -join ([char[]](0xD55C,0xAE00,0x20,0xAC00,0xB098,0xB2E4))
   $g.DrawString(('EMF 0123456789 '+$korean),$font,[Drawing.Brushes]::Black,50,20)
   $state = $g.Save()
   $g.SetClip([Drawing.RectangleF]::new(5,36,40,20))
   $g.FillEllipse([Drawing.Brushes]::OrangeRed,0,31,50,30)
   $g.DrawLine($blue,0,31,50,61)
   $g.Restore($state)
   $g.DrawRectangle($black,5,36,40,20)
   $state = $g.Save()
   $g.TranslateTransform(75,46); $g.RotateTransform(25)
   $g.FillRectangle([Drawing.Brushes]::RoyalBlue,-15,-5,30,10)
   $g.DrawString('ROTATE',$font,[Drawing.Brushes]::Black,-14,-3)
   $g.Restore($state)
   $g.FillRectangle([Drawing.Brushes]::Blue,103,36,18,18)
   $alpha = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(128,255,0,0)); $owned += $alpha
   $g.FillRectangle($alpha,112,43,18,18)
   $g.DrawString('RGB / clip / alpha',$font,[Drawing.Brushes]::Black,5,64)
   $tile = [Drawing.Bitmap]::new(16,16); $owned += $tile
   for($y=0;$y -lt 16;$y++){for($x=0;$x -lt 16;$x++){
    $color = if(($x -lt 8) -eq ($y -lt 8)){[Drawing.Color]::Black}else{[Drawing.Color]::Yellow}
    $tile.SetPixel($x,$y,$color)
   }}
   $g.DrawImage($tile,[Drawing.RectangleF]::new(110,65,18,10))
  } finally { $g.Dispose(); foreach($item in $owned){$item.Dispose()}; $meta.Dispose() }
  foreach($dpi in @(300,600)){
   $bytes=[IO.File]::ReadAllBytes($path)
   $frameWidth=[int64][BitConverter]::ToInt32($bytes,32)-[BitConverter]::ToInt32($bytes,24)
   $frameHeight=[int64][BitConverter]::ToInt32($bytes,36)-[BitConverter]::ToInt32($bytes,28)
   $w = [int][Math]::Round(142*$dpi/25.4); $h = [int][Math]::Round(142*$frameHeight/$frameWidth*$dpi/25.4)
   $bitmap = [Drawing.Bitmap]::new($w,$h); $bitmap.SetResolution($dpi,$dpi)
   $render = [Drawing.Graphics]::FromImage($bitmap)
   $input = [Drawing.Imaging.Metafile]::new($path)
   try {
    $render.Clear([Drawing.Color]::White)
    $render.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $render.TextRenderingHint = [Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $render.DrawImage($input,[Drawing.Rectangle]::new(0,0,$w,$h))
    $bitmap.Save((Join-Path $Directory ($variant+'-gdi-'+$dpi+'.png')),[Drawing.Imaging.ImageFormat]::Png)
   } finally { $input.Dispose();$render.Dispose();$bitmap.Dispose() }
  }
  Write-Output ('Created and independently rendered '+$path)
 }
} finally { $referenceGraphics.ReleaseHdc($dc);$referenceGraphics.Dispose();$reference.Dispose() }
