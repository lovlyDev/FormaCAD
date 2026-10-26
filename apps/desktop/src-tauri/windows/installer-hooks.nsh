; Installers before 1.2.x placed a single license FILE at licenses\occt. Current
; versions install a licenses\occt\ DIRECTORY, which NSIS cannot create while that
; old file exists ("Error opening file for writing"). Remove only that stale file.
!macro NSIS_HOOK_PREINSTALL
  IfFileExists "$INSTDIR\licenses\occt\*.*" formaOcctLicenseOk 0
    IfFileExists "$INSTDIR\licenses\occt" 0 formaOcctLicenseOk
      Delete "$INSTDIR\licenses\occt"
  formaOcctLicenseOk:
!macroend
