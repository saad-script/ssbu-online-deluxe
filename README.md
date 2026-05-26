# SSBU Online Deluxe (Beta)

A performance and networking enhancement mod for **Super Smash Bros. Ultimate** that introduces latency controls, render optimizations, and real-time online information.

> ⚠️ This is a **beta release**. Features and stability may change as development continues.
> ⚠️ Use at your own risk. I have been testing this mod online personally without any major issues, but there is still a non-zero risk of a ban. The overclocks are intentionally minimal; however, any hardware damage or account penalties remain your responsibility.

## 🚧 Status

This mod is currently in beta. The codebase will be cleaned up and published soon, along with a more detailed guide and documentation.

## ✅ Compatibility

- ✔️ Nintendo Switch (console)
- ✔️ Eden Emulator (requires workaround):
  - Right click SSBU -> Click `Configure Game` -> Click `System` tab -> Check `RNG Seed` -> Set to `00000000`
- ⚠️ Other emulators: not yet tested (not planned)
- ⚠️ HDR support not yet tested (planned)

## 📦 Installation

- Download and extract the zip from the releases, then:

- Console:
  - Copy the `atmosphere` folder to the root of the SD card.
  - Your Switch may need a full restart for the mod to load correctly.

- Emulator:
  - Copy the `atmosphere` folder to `%EDEN_FOLDER%/sdmc`. (Replace `%EDEN_FOLDER%` with whereever your eden folder is located)
  - Delete `lbnx_over.nro` from the `romfs` folder

## 🎮 Controls

### Toggle UI Modes
```
ZL + ZR + D-Pad Down
```

Cycles through:
- Hidden
- Full Info
- Performance Info

### Navigation (Full Info Mode)

- `D-Pad Up / Down` → Select option
- `D-Pad Left / Right` → Change value
- See below for an explanation on what these options do

## ✨ Features

### 🌐 Online Enhancements
- Display **opponent ping** in all online modes (including Elite Smash)
- Show **extended opponent info** *(only if both players have the mod)*:
  - Network latency / connection quality  
  - Opponent’s current network/render settings (latency slider, buffer mode, vsync status, etc.)

### 🎛️ Online Latency Controls
*(Available in Arena and Local Online modes only)*

- Adjust:
  - Latency value: Applied when entering a valid online match

### 🎛️ Render Optimizations Controls (for less input latency)
*(Available in Offline, Online Arena and Local Online modes only)*

**I would highly recommend you these options as is. The best settings are already applied automatically when entering/exiting a valid online match. These are for those who want to experiment and try out different settings**
- Adjust:
  - Buffer Mode
    - Triple (vanilla) => less frame drops, more stable, higher latency
    - Double (recommended for online) => lower latency, but may drop frames on intensive moves/effects
  - Index Mode:
    - 2 behind (vanilla) => Presented frame is 2 frames behind the rendered frame. 
    - 1 behind (recommended for console online) => Presented frame is 1 frame behind the rendered frame
    - Immediate (recommended for emu online) => Presented frame is 0 frames behind the rendered frame
  - Vsync:
    - Enabled (vanilla): higher latency, no screen tearing
    - Disabled (recommended for online): lower latency, but may cause screen tearing
  - Render optimizations:
    - Enabled (recommended for online): Reorder smashes render and polling system to optimize for input delay
    - Disabled (vanilla)
  - Dynamic Resolution:
    - Enabled (recommended for online) => dynamically lower resolution on intensive moves/effects to keep framerate stable
    - Disabled (vanilla)
  - Default Resolution:
    - baseline resolution of ssbu. Lower means less frame drops.

Recommended for offline:
  - triple buffer, index 2 behind, vsync enabled, render opts disabled, dynamic res disabled, default res 1920x1080p

Recommended for console online:
  - double buffer, index 1 behind, vsync disabled, render opts enabled, dynamic res enabled, default res 1920x1080p

Recommended for emulator online:
  - double buffer, index immediate, vsync disabled, render opts enabled, dynamic res enabled, default res 1920x1080p

### ⚡ Automatic Optimization
- Applies **recommended render settings automatically** when entering a valid online match
- Reverts to **vanilla settings** after exiting

> ⚠️ **Important:**  
> If you want to test custom settings, make sure to change them **after the online match starts**.
> Any settings configured before entering a valid match will be **overwritten by the automatic optimization**.
> Setting up automatic profiles for online/offline matches may be addressed in a future update

## 🙌 Credits

Huge thanks to the following people who made this possible. Without these people, this project wouldn't have been possible:

- **Bludev**
  For the intial SSBU render system research and the less-lag mod

- **BlankMausser**
  Creator of the SsbuSync mod, which this mod uses to modify ssbu's render system.
  BlanksMausser's work and guidance on SSBU’s rendering internals were critical to making this mod possible.

- **Kinnay** & contributors of the NintendoClients repo/wiki
  For guidance on network service implementation. The network service wouldn't have been possible if not for the incredible efforts of these people.

- **Coolsonickirby**
  For the imgui-smash plugin, making UI development significantly easier

