# Launching the Mac test

This experimental package requires macOS 11 or newer on Apple Silicon or Intel.
It was cross-built on Linux and has not been verified on a Mac. It is not
notarized, so macOS can show an "Apple could not verify" warning when opening it.

## First launch

1. Extract the entire ZIP into a writable folder. Keep `RUN.command`, both
   executables, `libredguard-glide.dylib`, `Frameworks/` and `game/` together.
   The personal test download includes the owned game files and disc media.
2. If macOS blocks the download, dismiss the warning with **Done**. For this
   trusted test package, open Terminal and remove the quarantine attribute from
   the extracted package folder:

   ```sh
   xattr -dr com.apple.quarantine "$HOME/Downloads/Maiko Mac Test"
   ```

   Change the path if you extracted it elsewhere. This recursively removes the
   download quarantine flag from this folder, including its bundled libraries.
   It does not sign or notarize the package. Use it only for a package you trust.
3. Double-click `RUN.command`, or launch it from Terminal:

   ```sh
   cd "$HOME/Downloads/Maiko Mac Test"
   ./RUN.command
   ```

The launcher chooses `redguard-native-arm64` on Apple Silicon and
`redguard-native-x86_64` on Intel. It supplies the adjacent `game/` folder explicitly. Use `RUN.command` with the
original test ZIP: that engine executable does not discover `game/` when opened
directly. New builds also discover that folder without launcher arguments.

## Troubleshooting

If Terminal reports `permission denied` for the launcher, restore its executable
permission with `chmod +x RUN.command` from the package folder, then retry.
If the same error names an engine executable, restore its permission with
`chmod +x redguard-native-arm64 redguard-native-x86_64`.

To capture startup output, run these commands in the package folder:

```sh
./RUN.command > mac-test.log 2>&1
```

To try starting without the intro movie:

```sh
./RUN.command --skip-intro > mac-test.log 2>&1
```

If it fails, share `mac-test.log`, your macOS version, whether your Mac uses Apple
Silicon or Intel, and what appeared on screen. A `dyld` missing-library error
usually needs the complete package extracted with its framework symlinks intact.
Keep the renderer and frameworks beside the executables.

## Test limits

The same CPU-state backend has rendered the original menu and opening ship on
Linux using software OpenGL and dummy audio. Mac graphics, input, hardware audio
and full-game completion remain unverified. On Linux, the longer movie probe
reaches the ship but times out at its final Quit action on both the offset and
existing native backends. The fresh book-menu Quit check passes.

Original Redguard assets remain proprietary and are included only in the owner's
authorized personal test download. See the package's licence notices and the
separate renderer source archive.
