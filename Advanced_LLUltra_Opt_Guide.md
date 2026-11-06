# SSBU Online Deluxe — LLUltra Advanced Optimization Guide

This guide is for players who want to use the **LLUltra profile on real Nintendo Switch hardware**. It walks through the settings needed to get the best LLUltra experience and to **eliminate the stutters and frame drops that can occur on console**.

> **Note:** This guide does not apply to **emulators**.

> ⚠️ **Use at your own risk.** The overclocks in this guide are intentionally conservative and should be safe for nearly all consoles. That said, overclocking always carries some risk, and that risk grows the further you push beyond the hardware's limits. In particular, setting RAM clocks too high can **corrupt your sysNAND/emuNAND**, so **back up your sysNAND/emuNAND before continuing**. You are solely responsible for any damage to your hardware or data.

## Hardware Differences

- **Erista** refers to the original/base v1 Nintendo Switch model.
- **Mariko** refers to the newer hardware revision, the OLED model.
- Mariko models are capable of being overclocked to higher memory clocks, so the instructions differ depending on your hardware.

### Prerequisites

Make sure that **Ultrahand Overlay** and **Horizon OC (HOC)** are installed properly.

See the installation instructions here:

https://github.com/Horizon-OC/Horizon-OC#installation

> **Hekate users:** Make sure you properly configure the `hekate_ipl.ini` file as described in the installation instructions above.

---

## 1. Optimizing RAM clocks (Highly Recommended)

This should account for the **majority of the stutters** you experience.

### Set the Max RAM Overclock

We need to allow setting ram clocks above the stock limit of 1600. This is safe as long you dont end up setting the clocks too high. The clocks I show here is a very minor overclock and will be fine with 99% of consoles.

1. Open the **HOC Overlay** using:
   - `ZL + ZR + D-Pad Down`

2. Navigate to:
   - **Settings → RAM Settings → RAM Max Clock**

3. Set the RAM Max Clock to:
   - **Erista:** `1833`
   - **Mariko:** `2131`
   - This doesnt change anything, it just allows you to set a higher clock (in step 2) than what is allowed by the console. You can even set it higher if you want to play around with higher overclocks later

4. **Restart the console.**

### Verify the Setting

After restarting:

1. Open the **HOC Overlay** again.
2. Select **Edit App Profile → Memory**.
3. Do **not** select anything.
4. Check whether you have the option to set the memory clock up to:
   - **1833 MHz on Erista**
   - **2131 MHz on Mariko**

If you **don't see an option that goes that high**, make sure the previous steps were completed correctly.

> **Important:** Pay particular attention to the Hekate configuration mentioned in the installation instructions if you're using Hekate.

---

## 2. Configure SSBU Online Deluxe (Highly Recommended)

Now we'll configure **SSBU Online Deluxe** to use the new RAM overclock when in a LLUltra match.

Create the following file on your SD card (create the folders if they dont exist):

```
sd:/ultimate/ssbu_online_deluxe/config.toml
```
Now write this into it:

Erista:
```
[render_config.render_profile_config.overrides.LLUltra]
overclock = { cpu = 1785, gpu = 921, mem = 1833 }
default_resolution = "576p"
dynamic_resolution = "480p"
```

Mariko:
```
[render_config.render_profile_config.overrides.LLUltra]
overclock = { cpu = 1785, gpu = 921, mem = 2131 }
default_resolution = "720p"
dynamic_resolution = "576p"
```

You can play around with setting the clocks and resolution to your liking. The above settings should be good enough to solve the majority of stutters though.

Verify your overclock overrides are actually applying in match:
- Load into a online match, and open the Horizon OC overlay:
- Navigate to: **Temporary Overrides**
- You should see the clocks you set in the config file.
- When you leave the match or close the game, the temporary overrides will clear.

---

## 3. Optimizing Controller Polling (Highly Recommended)

By default, the Switch polls controllers at 200 Hz (or 125 Hz, depending on the controller). At LLUltra's lower latency, this polling rate can occasionally cause inputs to be dropped during online play.

To fix this, we highly recommend raising the controller polling rate to **1000 Hz**. There are essentially no downsides to doing so.

Follow the instructions here to set your controller polling rate to 1000 Hz:  
https://github.com/saad-script/hidpoll

---

## 4. Optimizing GPU usage (Optional)

- The steps mentioned above should solve the majority of the stutters you experience on LLUltra.
- Avoid using stage/skin mods that increase the model details or have higher polycounts. This may cause stutters.
- On the other hand, if you use stage/skin mods that do the opposite (lower poly count, lower level of detail) it may actually help with performance.
   - For example, unmodded Hollow Bastion causes frame drops on LLUltra due to the background's polycount being too much for the GPU to handle in LLUltra mode. But if you use a stage mode like [Eternal Heart](https://gamebanana.com/mods/384225) which replaces the background, it actually fixes the stutters.
- Certain effects from certain moves may also cause framedrops and stutters. These moves can also be target with effect mods that replaces or simplifies them. 
- We may eventually put together a curated list of recommended stage, skin, and effect mods that work well alongside SSBU Online Deluxe.

## 5. Optimizing Temperatures (Optional)

- Generally, speaking anything under SOC temps of 70 degrees celsius is fine.
- The overclocks mentioned above should keep you well under that threshold.
- Though if you want still want to get lower temperatures, you can easily undervolt the CPU:
  - **HOC Overlay > Settings > CPU Settings > CPU UV Mode Slider**
  - Set it to the highest value that is stable on your switch
  - Example: Start with `CPU UV = 5`, if the switch crashes or acts unstable, restart, and set it to 4, and continue down until you find a stable value.
  - Undervolting is completely safe. You dont have to worry about damaging hardware.

## 6. Go down the OC Rabbit Hole (Optional, advanced users only)
- For those wanting to go further (gpu undervolt, ram latency, etc). You can find more information here:
  - Erista Advanced HOC Setup: https://rentry.co/erista
  - Mariko Advanced HOC Setup: https://rentry.co/mariko
