; Find the process whose image is the installed executable. File scanners can also
; appear in Restart Manager's resource list, so that list cannot identify the app.
!macro AUDIOFLOW_FIND_PROCESS executablePath result
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  StrCpy ${result} 0
  System::Call 'kernel32::CreateToolhelp32Snapshot(i 2, i 0) p .r0'
  ${If} $0 P<> -1
    ; NSIS is a 32-bit process. PROCESSENTRY32W has a 556-byte layout here.
    System::Alloc 556
    Pop $1
    ${If} $1 P<> 0
      System::Call '*$1(i 556)'
      System::Call 'kernel32::Process32FirstW(p r0, p r1) i .r2'
      ${DoWhile} $2 <> 0
        IntOp $3 $1 + 8
        System::Call '*$3(i .r3)'
        ; Query, synchronize, and terminate rights apply only to a matching app handle.
        System::Call 'kernel32::OpenProcess(i 0x101001, i 0, i r3) p .r4'
        ${If} $4 P<> 0
          StrCpy $5 ${NSIS_MAX_STRLEN}
          System::Call 'kernel32::QueryFullProcessImageNameW(p r4, i 0, w .r6, *i r5) i .r7'
          ${If} $7 <> 0
          ${AndIf} $6 == "${executablePath}"
            StrCpy ${result} $4
            ${ExitDo}
          ${EndIf}
          System::Call 'kernel32::CloseHandle(p r4)'
        ${EndIf}
        System::Call 'kernel32::Process32NextW(p r0, p r1) i .r2'
      ${Loop}
      System::Free $1
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p r0)'
  ${EndIf}
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

; Replace the bundled file-owner check for both installer and uninstaller.
!macroundef CheckIfAppIsRunning
!macro CheckIfAppIsRunning executablePath productName
  !define AUDIOFLOW_CHECK_ID ${__LINE__}
  Push $R0
  Push $R1
  Push $R3
  !insertmacro AUDIOFLOW_FIND_PROCESS "${executablePath}" $R3
  ${If} $R3 P<> 0
    StrCpy $R0 258
    ${If} $UpdateMode = 1
      DetailPrint "Waiting for AudioFlow to finish exiting..."
      System::Call 'kernel32::WaitForSingleObject(p R3, i 20000) i .s'
      Pop $R0
    ${EndIf}
    ${If} $R0 <> 0
      ${If} $PassiveMode <> 1
      ${AndIfNot} ${Silent}
        nsis_tauri_utils::StrReplace "$(appRunningOkKill)" "{{product_name}}" "${productName}"
        Pop $R1
        MessageBox MB_OKCANCEL $R1 IDOK audioflow_close_${AUDIOFLOW_CHECK_ID}
        System::Call 'kernel32::CloseHandle(p R3)'
        Abort
      ${EndIf}
      audioflow_close_${AUDIOFLOW_CHECK_ID}:
      ; The open handle keeps process identity stable even if a PID is reused.
      System::Call 'kernel32::TerminateProcess(p R3, i 0) i .s'
      Pop $R0
      System::Call 'kernel32::WaitForSingleObject(p R3, i 5000) i .s'
      Pop $R0
      ${If} $R0 <> 0
        System::Call 'kernel32::CloseHandle(p R3)'
        nsis_tauri_utils::StrReplace "$(failedToKillApp)" "{{product_name}}" "${productName}"
        Pop $R1
        Abort $R1
      ${EndIf}
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p R3)'
  ${EndIf}
  Pop $R3
  Pop $R1
  Pop $R0
  !undef AUDIOFLOW_CHECK_ID
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    ; A second instance forwards this argument to the tray process and exits.
    ; With no running instance, the application exits before creating its window.
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --quit-for-install'
    Push $R3
    !insertmacro AUDIOFLOW_FIND_PROCESS "$INSTDIR\${MAINBINARYNAME}.exe" $R3
    ${If} $R3 P<> 0
      System::Call 'kernel32::WaitForSingleObject(p R3, i 20000)'
      System::Call 'kernel32::CloseHandle(p R3)'
    ${EndIf}
    Pop $R3
  ${EndIf}
!macroend
