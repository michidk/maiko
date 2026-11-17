Maiko native runtime for Windows

You need your own copy of The Elder Scrolls Adventures: Redguard. Keep these files together in a folder of their own that you can write to, not inside your game folder.

Double-click maiko-launcher.exe, the launcher. It looks for your game by itself: the folder it remembered from an earlier start, the REDGUARD_DIR setting, its own folder and the folders around it, and the usual GOG folders. If it finds none, it asks once for the folder that contains RGFX.EXE and remembers your choice. It then shows Redguard's original start window, drawn from the REDGUARD.EXE in your installation. Game Options sets fullscreen, the window size and skipping the intro; Play Redguard starts the game.

You can also start redguard-native.exe directly. Options such as --fullscreen or --window-size 1600x1200 can be given on a Command Prompt line; the list is in the project README.

The game keeps its saves and settings in the game folder, shared with the GOG/DOSBox version.
A log of every start is kept under %LOCALAPPDATA%\Maiko\logs. Attach the newest one when you report a problem.

No DOSBox is needed. Reports are welcome at https://github.com/michidk/maiko/issues
