<h1 align="center">🦵 gmcl_rekinect</h1>
<p align="center">
	This is a reimplementation of Kinect support for Garry's Mod, allowing you to do stuff like <a href="https://youtu.be/PFkju1-0lZI" target="_blank">this</a> on more platforms.
</p>

# Features

* Support for the Xbox 360 Kinect
* Support for the Xbox One Kinect
* Support for the x86-64 branch of Garry's Mod

## Caveats:

* gmcl_rekinect does not modify the menu state, so the Kinect icon in the bottom right of the Gmod menu will not be visible.
* `motionsensor.GetColourMaterial()` is not implemented.

# Requirements

* Windows
* For Xbox 360 Kinect users, [Kinect for Windows Runtime 1.8](https://www.microsoft.com/en-us/download/details.aspx?id=40277)
* For Xbox One Kinect users, Windows 10 or Windows 11

# Installation

1. Open your Garry's Mod installation directory. You can find this by right clicking Garry's Mod on Steam, clicking "Properties", clicking the "Installed Files" tab, and then clicking "Browse".
2. Navigate into the `garrysmod/lua/` folder.
3. Create a new folder called `bin`. If it already exists, skip this step.
4. Download the latest release of `gmcl_rekinect.zip` from the [Releases](https://github.com/WilliamVenner/gmcl_rekinect/releases) page.
5. Drop all of the files inside the .zip file into the `garrysmod/lua/bin/` folder you just created.

Your `bin` folder should now look something like this:

todo

# Usage

## Using `rekinector` (recommended)

gmcl_rekinect comes with a custom DLL injection program for your convenience.

Just open it and it'll watch for Garry's Mod to open. Once it does, it'll inject gmcl_rekinect into the process.

[Download](https://github.com/WilliamVenner/gmcl_rekinect/releases)

## As a binary module

gmcl_rekinect can be used as a clientside binary module. Some servers will allow you to simply run this console command:

```lua
lua_run_cl require("rekinect")
```

in order to load gmcl_rekinect. If nothing is printed in your console after running this command, you'll need to use [`rekinector`](#using-rekinector-recommended) [DLL injection](#dll-injection) instead.

## DLL injection

gmcl_rekinect can also be directly injected into the Garry's Mod process, allowing you to use it on whatever servers you please.

If you don't know how to do this already, follow these instructions:

1. Make sure you have followed the [Installation Instructions](#installation) above
2. Install <a href="https://www.cheatengine.org/downloads.php" target="_blank">Cheat Engine</a>[]
3. If you're connected to a server - disconnect. **DLL injection of gm_rekinect must take place before joining a server.**
4. Open Cheat Engine
5. Click "Select a process to open" in the top left
6. Select the "Applications" tab and click "Garry's Mod"
7. Click "Open"
8. Click "Memory View" in the middle left of the Cheat Engine window
9. Click "Tools" at the top of the Memory View window
10. Click "Inject DLL"
11. If you're on the main branch of Garry's Mod (the default branch), inject `gmcl_rekinect_win32.dll` which you installed to `garrysmod/lua/bin/` earlier. Otherwise, x86-64 branch users should inject `gmcl_rekinect_win64.dll`.
12. Cheat Engine will ask if you want to execute a function of the DLL. Click "No".
13. Join a server.
