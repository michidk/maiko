@echo off
setlocal DisableDelayedExpansion
rem Folder containing your original RGFX.EXE and game assets, for example:
rem   set "GAME_DIR=C:\GOG Games\Redguard\Redguard"
rem Leave it empty to choose the folder in a dialog each time.
set "GAME_DIR="
rem Display: leave empty for a window at the game's own size (1024x768), or set one of
rem   set "DISPLAY_ARGS=--fullscreen"
rem   set "DISPLAY_ARGS=--window-size 1600x1200"
set "DISPLAY_ARGS="
rem Keep copied saves/settings beside this runtime, separate from the original game.
set "USER_DIR=%~dp0user-data"
rem Optional startup options, for example: set "OPTIONS=--skip-intro --skip-outro --load-save 1"
set "OPTIONS="
if not "%~1"=="" set "GAME_DIR=%~1"
cd /d "%~dp0"
if defined GAME_DIR (
  redguard-native.exe --game-dir "%GAME_DIR%\." --user-dir "%USER_DIR%" %DISPLAY_ARGS% %OPTIONS% > native-test.log 2>&1
) else (
  redguard-native.exe --choose-game-dir --user-dir "%USER_DIR%" %DISPLAY_ARGS% %OPTIONS% > native-test.log 2>&1
)
set "native_exit=%errorlevel%"
>>native-test.log echo Exit code: %native_exit%
type native-test.log
echo.
echo Original saves/settings are copied to "%USER_DIR%" on first run.
echo Installed GOG game assets and bundled movie data use GAME_DIR automatically.
echo Diagnostics are saved in native-test.log.
pause
exit /b %native_exit%
