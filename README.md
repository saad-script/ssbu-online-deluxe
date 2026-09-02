# SSBU Online Deluxe

A performance and online enhancement mod for **Super Smash Bros. Ultimate** that introduces latency controls, render optimizations, and real-time online information.


> ⚠️ SSBU 13.0.5 does NOT change how packets are sent, received, or parsed. This mod operates exclusively at the P2P networking layer, meaning it modifies traffic exchanged directly between players. The game server is not involved in this communication and, under normal operation, should have no visibility into what this mod is doing. There is currently no known indication that 13.0.5 introduced additional checks for invalid or modified P2P packet data that could result in a ban. The risk of using this mod on 13.0.5 is therefore the same as on previous versions. Please be patient while we update this mod to work with version 13.0.5.  

> ⚠️ This is a work in progress. Features and stability may change as development continues.  
> ⚠️ Use at your own risk. I have been testing this mod online personally without any major issues, but there is still a non-zero risk of a ban. The overclocks are intentionally minimal; however, any hardware damage or account penalties remain your responsibility.  

## ✅ Compatibility

- ✔️ Nintendo Switch (console)
- ✔️ Eden Emulator (requires workaround, see installation section below)
- ⚠️ Other emulators: not yet tested (not planned)
- ⚠️ HDR support not yet tested (planned)

## 📦 Installation

> ⚠️ Remove any previous latency slider mod, vsync mod, and less lag mod before proceeding with the installation steps!

### Manual Installation

- Ensure you have these prerequisite installed on your switch/emulator:
  - ~~[skyline](https://github.com/skyline-dev/skyline/releases)~~
    - ⚠️ The latest version causes crashes. Use the version bundled into the ssbu-online-deluxe release zip.
  - [arcropolis](https://github.com/raytwo/arcropolis/releases)
  - [nro-hook](https://github.com/ultimate-research/nro-hook-plugin/releases)
  - [smashline](https://github.com/HDR-Development/smashline/releases)
  - [imgui-smash](https://github.com/Coolsonickirby/imgui-smash/releases)
  - [ssbu-pia-manager](https://github.com/project-ultelier/ssbu-pia-interface/releases)
  - ~~[ssbusync](https://github.com/project-ultelier/smash-ultelier/releases)~~
    - ⚠️ Currently outdated. Use the version bundled into the ssbu-online-deluxe release zip.
- Then you can install the latest release of ssbu-online-deluxe: [ssbu-online-deluxe](https://github.com/saad-script/ssbu-online-deluxe/releases)


### Automatic Installation

Console:
- From the releases page, download `create-sdcard-folder.zip` and then run `create-sdcard-folder.bat`. On linux, you can install powershell for your distro and run `create-sdcard-folder.ps1`. It will download and setup the atmosphere folder for you in a newly created folder `sdcard/`. Then copy the contents of `sdcard/` to the root of your SD card.
- Alternatively, you can use the app I made: [ssbu-emu-optimizer](https://github.com/saad-script/ssbu-emu-optimizer/releases). Install, then click `Generate SDCard Folder`, then copy the generate folder contents to the root of the sd card.

Emulator:
- From the releases page, download `create-sdcard-folder.zip` and then run `create-sdcard-folder.bat`. On linux, you can install powershell for your distro and run `create-sdcard-folder.ps1`. It will download and setup the atmosphere folder for you in a newly created folder `sdcard/`. Then copy the contents of `sdcard/` to your `eden/sdmc` folder.
  - Then, apply this workaround if you are on Eden emulator:
    - Right click SSBU -> Click `Configure Game` -> Click `System` tab -> Check `RNG Seed` -> Set to `00000000`
- Alternatively, you can use the app I made: [ssbu-emu-optimizer](https://github.com/saad-script/ssbu-emu-optimizer/releases). Install, and then configure it to point to the correct eden folder, then check `SSBU Settings`, `SSBU Mods`, `Save Data` (if you want a 100% save), then click optimize.


### Verify

Verify that your sdcard directory strucure looks like this on your switch or emulator:

```
`sdcard/` (or `sdmc/` on emulator)
│
├── atmosphere/
│   └── contents/
│       ├── 00FF0000A11CE0FF/
│       │   ├── exefs.nsp
│       │   └── flags/
│       │       └── boot2.flag
│       └── 01006A800016E000/
│           ├── exefs/
│           │   ├── main.npdm
│           │   └── subsdk9
│           └── romfs/
│               └── skyline/
│                   └── plugins/
│                       ├── libarcropolis.nro
│                       ├── libimgui_smash.nro
│                       ├── libnro_hook.nro
│                       ├── libnx_over.nro
│                       ├── libsmashline_plugin.nro
│                       ├── libssbu_online_deluxe.nro
│                       ├── libssbu_pia_manager.nro
│                       └── libssbusync.nro
│
```

## 🎮 Controls

### Native UI (Online Character Select Screen and Online Arena)

> Note: `All Shoulder Buttons` = `L + R + Z` on gamecube controller, `ZL + ZR + L + R` on procontroller

- On the character select screen or online arena:
  - `D-pad Left/Right`: Select network latency
  - `D-pad Up/Down`: Select render profile
  - `All Shoulder Buttons + X`: Toggle FPS Boost Mode (AKA FPS++ mode)
  - `All Shoulder Buttons + Y`: Toggle Streamer Mode (show/hide custom native ui)

- On the character select screen (more than one opponent):
  - `Left Trigger + Right Trigger + Dpad Left/Right`: Cycle between which opponent's network info to show

See 'Features' section below to see what these options do

### Overlay UI (Optional)

- `Left Trigger + Right Trigger + D-Pad Down` → Cycle between current window mode
  - Window Modes: `Hidden`, `Full Info`, `Performance Info`
  - In `Full Info Mode`:
    - `D-Pad Up / Down` → Select row
    - `D-Pad Left / Right` → Change value
    - While row `NetProfile` is selected:
      - `All Shoulder Buttons + X`: Toggle FPS Boost mode (AKA FPS++ mode)

See 'Features' section below to see what these options do

## ✨ Features

### 🌐 Online Enhancements

- Display **opponent ping** in all online modes (including Elite Smash):
  - Network RTT (ping) / connection quality
  - Green=Stable, Yellow=Inconsistent, Red=Unstable
- Show **extended opponent info** *(only if both players have the mod)*:
  - Opponent’s current network/render settings (latency slider, render profile)

### 🎛️ Online Latency Controls
*(Available in Online Arena and Local Online modes only)*  
*(Available in Quickplay/Elite on Nextendo servers only)*

- This allows you to control the online latency delay frames.
- Adjust:
  - Latency value:
    - Auto: Applies SSBU's default latency calculation method.
    - 0f-25f: Manually set the latency delay frames

> It is recommended to manually set the latency delay frames based on the ping and connection quality.

### 🎛️ Render Profile Controls
*(Available in Online Arena and Local Online modes only)*  
*(Available in Quickplay/Elite on Nextendo servers only)*

- This allows you to set the games render/graphic settings for less native input delay.
- Adjust:
  - Render Profile:
    - Auto: Applies the recommended profile based on platform (console/emulator) and number of players.
    - Vanilla: This is the default vanilla profile that the game uses by default.
    - LessLag: This applies optimizations to cut 3 frames of native input delay.
    - LLUltra: This applies optimizations to cut 4 frames of native input delay.
      - This also works on console, but the game resolution will be scaled down to keep it stutter free.
      - On console, you may notice that certain UI elements look glitchy, such as the fighter cut-in screen, and match start countdown ui.
    - LLDoubles (Recommended for doubles): This applies optimizations to cut 2 frames of native input delay. This should work even in doubles when there are alot of players on screen without stuttering.
  - FPS Boost mode (AKA FPS++ Mode):
    - Only available on emulators.
    - If enabled, the current profile has '++' at the end of it. For example: LLUltra++
    - The amount of native latency it reduces varies based on the currently selected profile. For example, this will cutoff 3f of delay on Vanilla profile. But on LLUltra, it will only cutoff about half a frame of delay.
    - This may introduce some frametime variance causing the game to not feel as smooth.

**If you arent sure what profile to use, just leave it on Auto**

Best profile for console:
  - LessLag or LLUltra (depending on preference)

Best profile for emulator:
  - LLUltra

Best profile for doubles:
  - LLDoubles

> The mod will apply the **selected render profile automatically** when entering a valid online match.  
> Reverts to **vanilla settings** after exiting  
> You can play offline/training modes without having to worry about timing differences.

### 🎛️ Render Profile Config (Optional)

You can specify a config file in `sd/ultimate/ssbu_online_deluxe/config.toml`
- This will allow you to set the profile to use in the menu, and offline singles/doubles matches
- Add '++' at the end of the profile name to enable fps boost mode (emulator only)
- If you already have an overclock sysmodule, and dont want to conflict with or use ssbu-online-deluxe's built in overclocker:
  - Set `overclocker = false` in config file
  - Delete `libnx_over.nro` plugin file
  - Delete `atmosphere/contents/00FF0000A11CE0FF/` sysmodule folder
  - Restart switch
- All fields are optional. If you dont specify a field, it will use the default/recommended value.

Example `config.toml`:
```
overclocker = true                          # Set to 'false' if you are using your own overclock sysmodule

[render_profile_config]
menu = "Vanilla"                            # Recommended to keep this on Vanilla always
offline_match.singles = "Vanilla"           # Applies to offline single matches (1 or 2 players)
offline_match.doubles = "Vanilla"           # Applies to offline doubles matches (more than 2 players)
online_match.singles = "LessLagUltra++"     # 'Auto' mode will choose this profile for online single matches
online_match.doubles = "LessLag"            # 'Auto' mode will choose this profile for online double matches
```

## 📝 Notes and Contribution

- The dynamic resolution logic currently only applies to zoom in moves (final hit/critical hit) and Sephiroth's gigaflare.  
- Contributions are open especially for applying dynamic resolution to moves that cause stutter. I don't know if I'll have time to optimize every single move, so if you notice a specific move causes stutters, you can use smashline's api to contribute and optimize the move. You can start by viewing how `src/perf_scaler` currently applies dynamic resolution optimization.

## 🙌 Credits

Huge thanks to the following people who made this possible. Without these people, this project wouldn't have been possible:

- **Bludev**
  For SSBU render system research and the initial less-lag and latency slider mod.

- **BlankMauser**
  Creator of the SsbuSync and smash-ultelier mod, which this mod uses to modify ssbu's render system.
  BlanksMauser's work and guidance on SSBU’s rendering internals were critical to making this mod possible.

- **Kinnay** & contributors of the NintendoClients repo/wiki
  For guidance on network service implementation. The network service wouldn't have been possible if not for the incredible efforts of these people.

- **Coolsonickirby**
  For the imgui-smash plugin, making UI development significantly easier

- **The HDR team**
  For smashline, allowing for easy figher/effect/moveset hooks and adjustments 

- **The developers of Skyline**
  For the modding environment, allowing for code hooking/edits
