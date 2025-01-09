# buoy

**buoy is a music client for a subsonic server written with rust and GTK4 designed for elementary OS.**

---

## Screenshot

![Screenshot of buoy](data/screenshots/dark.png)

## Install the needed depencies

```bash
sudo apt install libgtk-4-dev libgranite-7-dev libgstreamer1.0-dev
```

it is assumed, that rust is already installed.


## Building, running and installing with meson and ninja

Build with the following
```bash
meson setup build
cd build
ninja
```

Run with
```bash
./src/debug/buoy
```

It can be installed with the following command
```bash
ninja install
```

## Building with flatpak

Install dependencies
```bash
sudo apt install python3-aiohttp python3-toml
wget https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
```

Building the project
```bash
python3 ./flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
flatpak --user install -y --noninteractive io.elementary.Platform/x86_64/8 io.elementary.Sdk/x86_64/8
```

It should now be able to run with a program launcher or from command line with
```bash
flatpak run com.github.eppixx.buoy
```

