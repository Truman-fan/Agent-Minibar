# Helpers to drive and capture the real app for reference screenshots.
param([string]$Action, [int]$X, [int]$Y, [string]$Out, [string]$Title)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text;
public static class W {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
}
"@
[W]::SetProcessDPIAware() | Out-Null
switch ($Action) {
  "click" { [W]::SetCursorPos($X, $Y); Start-Sleep -m 60; [W]::mouse_event(2,0,0,0,[IntPtr]::Zero); [W]::mouse_event(4,0,0,0,[IntPtr]::Zero) }
  "move" { [W]::SetCursorPos($X, $Y) }
  "windows" {
    $procId = (Get-Process agent-minibar).Id
    [W]::EnumWindows({ param($h, $l)
      $p = 0; [W]::GetWindowThreadProcessId($h, [ref]$p) | Out-Null
      if ($p -eq $procId -and [W]::IsWindowVisible($h)) {
        $sb = New-Object Text.StringBuilder 256; [W]::GetWindowText($h, $sb, 256) | Out-Null
        $r = New-Object W+R; [W]::GetWindowRect($h, [ref]$r) | Out-Null
        "{0} '{1}' {2},{3},{4},{5}" -f $h, $sb, $r.L, $r.T, $r.Rt, $r.B
      }; $true }, [IntPtr]::Zero) | Out-Null
  }
  "shot" {
    $parts = $Title -split ','
    $l=[int]$parts[0]; $t=[int]$parts[1]; $w=[int]$parts[2]-$l; $h=[int]$parts[3]-$t
    $bmp = New-Object Drawing.Bitmap $w, $h
    $g = [Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($l, $t, 0, 0, $bmp.Size)
    $bmp.Save($Out, [Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  }
}
