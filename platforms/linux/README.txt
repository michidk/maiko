Maiko native runtime for Linux (x86-64)

You need your own copy of The Elder Scrolls Adventures: Redguard. Keep these files together in a folder of their own that you can write to, not inside your game folder.

Run ./maiko, the launcher, on an X11 or Wayland desktop with a Vulkan or OpenGL (EGL) driver. It needs glibc 2.36 or newer and the system's ALSA and udev libraries (libasound.so.2, libudev.so.1), which every desktop distribution installs.

The launcher looks for your game by itself: the folder it remembered from an earlier start, the REDGUARD_DIR setting, its own folder and the folders around it, and the usual GOG, Wine, Heroic and Lutris install folders. If it finds none, it asks for the folder that contains RGFX.EXE and remembers your choice. It then shows Redguard's original start window, drawn from the REDGUARD.EXE in your installation. Game Options sets fullscreen, the window size and skipping the intro; Play Redguard starts the game.

You can also start ./redguard-native directly, with --game-dir /path/to/Redguard and the options listed in the project README.

The game keeps its saves and settings in the game folder, shared with the GOG/DOSBox version.
A log of every start is kept under ~/.local/state/maiko/logs. Attach the newest one when you report a problem.

No DOSBox is needed. Reports are welcome at https://github.com/michidk/maiko/issues
