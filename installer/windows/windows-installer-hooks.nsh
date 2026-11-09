; Quiver links the MSVC runtime dynamically, and a fresh Windows does not ship
; it: without vcruntime140.dll the app dies at launch with 0xC0000135.
; This hook installs the Visual C++ 2015-2022 x64 redistributable when it is
; missing or older than the minimum below.
;
; The redistributable is embedded so installs work offline. QUIVER_VCREDIST_PATH
; must point at a vc_redist.x64.exe when the installer is compiled (CI fetches
; and verifies it in .github/actions/build-tauri). The build fails otherwise,
; rather than shipping an installer that cannot bring the runtime along.

!define VCRUNTIME_REGKEY "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"
!define VCRUNTIME_MIN_MINOR 38
!define VCRUNTIME_DOWNLOAD_URL "https://aka.ms/vs/17/release/vc_redist.x64.exe"

; Leaves 1 in $0 when a new enough runtime is installed, 0 otherwise.
Function QuiverVcRuntimeInstalled
  ; The installer is 32-bit, the runtime key is in the 64-bit registry view.
  SetRegView 64
  StrCpy $0 0
  ReadRegDWORD $1 HKLM "${VCRUNTIME_REGKEY}" "Installed"
  ReadRegDWORD $2 HKLM "${VCRUNTIME_REGKEY}" "Major"
  ReadRegDWORD $3 HKLM "${VCRUNTIME_REGKEY}" "Minor"
  ${If} $1 = 1
  ${AndIf} $2 = 14
  ${AndIf} $3 >= ${VCRUNTIME_MIN_MINOR}
    StrCpy $0 1
  ${EndIf}
  ; Only the 14.x line is expected to be newer, so a higher Major also counts.
  ${If} $1 = 1
  ${AndIf} $2 > 14
    StrCpy $0 1
  ${EndIf}
  SetRegView lastused
FunctionEnd

Function QuiverEnsureVcRuntime
  Call QuiverVcRuntimeInstalled
  ${If} $0 = 1
    Return
  ${EndIf}

  DetailPrint "Installing the Microsoft Visual C++ runtime"
  Delete "$TEMP\quiver_vc_redist.x64.exe"

  !if /FileExists "$%QUIVER_VCREDIST_PATH%"
    File "/oname=$TEMP\quiver_vc_redist.x64.exe" "$%QUIVER_VCREDIST_PATH%"
  !else
    !error "Set QUIVER_VCREDIST_PATH to a vc_redist.x64.exe (${VCRUNTIME_DOWNLOAD_URL})"
  !endif

  ; The redistributable needs admin rights. The "runas" verb raises a UAC
  ; prompt for this one step, so the rest of the install stays per-user.
  ; ExecShellWait does not return the exit code, so success is decided by
  ; checking the registry again. That also covers 1638 (a newer runtime is
  ; already there) and 3010 (installed, reboot pending), which both leave the
  ; runtime registered. No reboot is forced.
  ExecShellWait "runas" "$TEMP\quiver_vc_redist.x64.exe" "/install /quiet /norestart" SW_HIDE
  Delete "$TEMP\quiver_vc_redist.x64.exe"

  Call QuiverVcRuntimeInstalled
  ${If} $0 = 1
    DetailPrint "Microsoft Visual C++ runtime installed"
    Return
  ${EndIf}

  DetailPrint "The Microsoft Visual C++ runtime could not be installed"
  MessageBox MB_OK|MB_ICONEXCLAMATION "Quiver needs the Microsoft Visual C++ Redistributable (x64), and it could not be installed. Quiver will not start until it is.$\r$\n$\r$\nInstall it from ${VCRUNTIME_DOWNLOAD_URL} (or run: winget install Microsoft.VCRedist.2015+.x64), then start Quiver." /SD IDOK
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  Call QuiverEnsureVcRuntime
!macroend
