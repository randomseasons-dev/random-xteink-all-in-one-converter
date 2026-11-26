@echo off
call "D:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
set CARGO_BUILD_JOBS=1
set TEMP=D:\tmp-build
set TMP=D:\tmp-build
cd /d "D:\Github Projects\random-xteink-all-in-one-converter\xteink"
call npm run tauri build
