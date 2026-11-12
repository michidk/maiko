@echo off
setlocal DisableDelayedExpansion
rem Folder containing your original RGFX.EXE and game assets.
set "GAME_DIR=D:\Games\GOG Galaxy\Redguard\Redguard"
rem Keep copied saves/settings beside this runtime, separate from the original game.
set "USER_DIR=%~dp0user-data"
rem Optional movie-media image, only if you have one. Leave empty otherwise.
rem Installed menu artwork and textures do not need this setting.
set "CD_IMAGE="
if not "%~1"=="" set "GAME_DIR=%~1"
cd /d "%~dp0"
if defined GAME_DIR (
  if defined CD_IMAGE (
    redguard-native.exe --game-dir "%GAME_DIR%\." --user-dir "%USER_DIR%" --cd-image "%CD_IMAGE%" > native-test.log 2>&1
  ) else (
    redguard-native.exe --game-dir "%GAME_DIR%\." --user-dir "%USER_DIR%" > native-test.log 2>&1
  )
) else (
  if defined CD_IMAGE (
    redguard-native.exe --choose-game-dir --user-dir "%USER_DIR%" --cd-image "%CD_IMAGE%" > native-test.log 2>&1
  ) else (
    redguard-native.exe --choose-game-dir --user-dir "%USER_DIR%" > native-test.log 2>&1
  )
)
set "native_exit=%errorlevel%"
>>native-test.log echo Exit code: %native_exit%
type native-test.log
echo.
echo Experimental gameplay build. Full-game completion and real Windows acceptance remain unverified.
echo Original saves/settings are copied to "%USER_DIR%" on first run.
echo Installed menu artwork and textures use GAME_DIR. No CD image is required for them.
echo Optional movie media, hardware movie audio and synchronization need further testing.
echo Loose movies are searched in GAME_DIR and its MOVIES, ANIMS and VIDEO folders.
echo Jump and frame-timing diagnostic samples are included in native-test.log.
echo CD music remains unsupported.
pause
exit /b %native_exit%
