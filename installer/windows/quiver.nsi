; Quiver Desktop installer (NSIS, per-user: %LOCALAPPDATA%\Quiver, no elevation), built by
; `cargo xtask package windows`:
;   makensis /DVERSION=<v> /DSTAGE=<dir with the exes + icon.ico> /DOUT=<setup.exe> quiver.nsi
; Install location, uninstaller name ($INSTDIR\uninstall.exe) and silent switch (/S) are what the
; shipped ARROW.md lifecycles (detect, install, update, uninstall) rely on: do not change them.
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "pass /DVERSION=<version>"
!endif
!ifndef STAGE
  !error "pass /DSTAGE=<dir>"
!endif
!ifndef OUT
  !error "pass /DOUT=<installer path>"
!endif

!define PRODUCT "Quiver"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\com.rabbytesoftware.quiver"

Name "${PRODUCT}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\${PRODUCT}"
InstallDirRegKey HKCU "Software\rabbytesoftware\${PRODUCT}" ""
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "${STAGE}\icon.ico"
!define MUI_UNICON "${STAGE}\icon.ico"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\quiverdesktop.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; Installs the Visual C++ runtime when missing (see the hook file for why).
!include "windows-installer-hooks.nsh"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${PRODUCT} Desktop installer"

Section "Install"
  !insertmacro NSIS_HOOK_PREINSTALL
  SetOutPath "$INSTDIR"
  File "${STAGE}\quiverdesktop.exe"
  File "${STAGE}\quiver.exe"
  File "${STAGE}\icon.ico"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\quiverdesktop.exe" "" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\rabbytesoftware\${PRODUCT}" "" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Rabbyte Software"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\quiverdesktop.exe"
  Delete "$INSTDIR\quiver.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey /ifempty HKCU "Software\rabbytesoftware\${PRODUCT}"
SectionEnd
