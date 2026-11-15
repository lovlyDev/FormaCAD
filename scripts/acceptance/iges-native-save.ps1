param(
 [Parameter(Mandatory=$true)][int]$HostProcessId,
 [Parameter(Mandatory=$true)][string]$RuntimeDirectory,
 [Parameter(Mandatory=$true)][string]$Destination,
 [switch]$Cancel
)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
if($env:FORMA_NATIVE_ACCEPTANCE -ne 'isolated'){throw 'Isolated native acceptance required'}
$runtime=[IO.Path]::GetFullPath($RuntimeDirectory).TrimEnd('\')
$target=[IO.Path]::GetFullPath($Destination)
if(-not $target.StartsWith($runtime+'\',[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetExtension($target) -ne '.iges'){throw 'Destination must be a new IGES file inside this runtime'}
if(Test-Path -LiteralPath $target){throw 'Existing export is preserved'}
if([int](Get-Content -LiteralPath (Join-Path $runtime 'pid.txt') -Raw) -ne $HostProcessId){throw 'Runtime PID mismatch'}
$hostProcess=Get-Process -Id $HostProcessId
if($hostProcess.Path -ne (Join-Path $runtime 'forma-cad.exe')){throw 'Only the isolated host may be controlled'}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$processCondition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$HostProcessId)
$dialog=$null
$russianTitle=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('0KHQvtGF0YDQsNC90LjRgtGMIENBRC3QvNC+0LTQtdC70Yw='))
$deadline=[DateTime]::UtcNow.AddSeconds(40)
while([DateTime]::UtcNow -lt $deadline -and -not $dialog){
 $windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,$processCondition)
 foreach($window in $windows){if($window.Current.ClassName -eq '#32770' -and $window.Current.Name -in @('Save CAD model',$russianTitle)){$dialog=$window;break}}
 if(-not $dialog){Start-Sleep -Milliseconds 200}
}
if(-not $dialog){throw 'Native Save CAD model dialog was not found on the isolated host'}
Add-Type -TypeDefinition @'
using System;using System.Text;using System.Collections.Generic;using System.Runtime.InteropServices;
public static class FormaIgesSaveControls {
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int left,top,right,bottom;}
 [StructLayout(LayoutKind.Sequential)] public struct GuiInfo {public int size,flags;public IntPtr active,focus,capture,menuOwner,moveSize,caret;public Rect caretRect;}
 public delegate bool Callback(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr h,Callback c,IntPtr p);
 [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder s,int n);
 [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] public static extern IntPtr SetText(IntPtr h,uint msg,IntPtr w,string text);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] public static extern IntPtr ReadText(IntPtr h,uint msg,IntPtr w,StringBuilder text);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint msg,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint msg,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint t,ref GuiInfo i);
 public static bool Focus(IntPtr root,IntPtr control){
  SendMessage(root,0x0028,control,new IntPtr(1));
  uint p;var t=GetWindowThreadProcessId(root,out p);var i=new GuiInfo();i.size=Marshal.SizeOf(typeof(GuiInfo));
  return GetGUIThreadInfo(t,ref i)&&i.focus==control;
 }
 public static IntPtr Find(IntPtr root,int id,string type) {
  var found=new List<IntPtr>();
  EnumChildWindows(root,(h,p)=>{var name=new StringBuilder(256);GetClassName(h,name,256);if(GetDlgCtrlID(h)==id&&name.ToString()==type)found.Add(h);return true;},IntPtr.Zero);
  if(found.Count!=1)throw new Exception("Exact native control is missing or ambiguous: "+id);
  return found[0];
 }
}
'@
$root=[IntPtr]$dialog.Current.NativeWindowHandle
if(-not $Cancel){
 # Navigate through the actual address editor; WM_SETTEXT on the filename
 # alone does not update IFileDialog's selected path.
 $toolbar=[FormaIgesSaveControls]::Find($root,1001,'ToolbarWindow32')
 $addressDeadline=[DateTime]::UtcNow.AddSeconds(10)
 do{
  $addressBar=[System.Windows.Automation.AutomationElement]::FromHandle($toolbar)
  $addressName=[string]$addressBar.Current.Name
  if($addressName -and $addressBar.Current.BoundingRectangle.Width -gt 100){break}
  Start-Sleep -Milliseconds 100
 }while([DateTime]::UtcNow -lt $addressDeadline)
 if(-not $addressName){throw 'Native address provider is not ready'}
 if(-not $addressName.EndsWith($runtime,[StringComparison]::OrdinalIgnoreCase)){
  $width=$addressBar.Current.BoundingRectangle.Width
  $point=[IntPtr]((15 -shl 16) -bor [int]($width*0.8))
  [void][FormaIgesSaveControls]::SendMessage($toolbar,0x0201,[IntPtr]1,$point)
  [void][FormaIgesSaveControls]::SendMessage($toolbar,0x0202,[IntPtr]::Zero,$point)
  $address=[FormaIgesSaveControls]::Find($root,41477,'Edit')
  if(-not [FormaIgesSaveControls]::IsWindowVisible($address)){throw 'Native address editor is unavailable'}
  if(-not [FormaIgesSaveControls]::Focus($root,$address)){throw 'Address editor does not own keyboard focus; navigation refused'}
  [void][FormaIgesSaveControls]::SetText($address,0x000C,[IntPtr]::Zero,$runtime)
  [void][FormaIgesSaveControls]::PostMessage($address,0x0100,[IntPtr]13,[IntPtr]0x1C0001)
  [void][FormaIgesSaveControls]::PostMessage($address,0x0101,[IntPtr]13,[IntPtr]::Zero)
  $folderDeadline=[DateTime]::UtcNow.AddSeconds(10)
  do{
   $addressName=[string][System.Windows.Automation.AutomationElement]::FromHandle($toolbar).Current.Name
   if($addressName.EndsWith($runtime,[StringComparison]::OrdinalIgnoreCase)){break}
   Start-Sleep -Milliseconds 100
  }while([DateTime]::UtcNow -lt $folderDeadline)
  if(-not $addressName.EndsWith($runtime,[StringComparison]::OrdinalIgnoreCase)){throw 'Actual native destination folder is not the isolated runtime'}
 }
 $edit=[FormaIgesSaveControls]::Find($root,1001,'Edit')
 $filenameBuffer=[Text.StringBuilder]::new(1024)
 [void][FormaIgesSaveControls]::ReadText($edit,0x000D,[IntPtr]1024,$filenameBuffer)
 $filename=$filenameBuffer.ToString()
 if($filename -ne [IO.Path]::GetFileName($target) -and $filename -ne [IO.Path]::GetFileNameWithoutExtension($target)){throw 'Native default filename differs from the expected new export'}
 if(Test-Path -LiteralPath $target){throw 'Existing export is preserved'}
}
$button=[FormaIgesSaveControls]::Find($root,$(if($Cancel){2}else{1}),'Button')
if(-not [FormaIgesSaveControls]::IsWindowEnabled($button) -or -not [FormaIgesSaveControls]::IsWindowVisible($button)){throw 'Native button is unavailable'}
if(-not [FormaIgesSaveControls]::PostMessage($button,0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)){throw 'Native button activation failed'}
Write-Output $(if($Cancel){'Native Windows save dialog canceled'}else{'Native Windows save dialog completed'})
