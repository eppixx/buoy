# buoy

**buoy is a music client for a subsonic server written with rust and GTK4 designed for elementary OS.**

---

## Install the needed depencies

```bash
sudo apt install libgtk-4-dev libgranite-7-dev libgstreamer1.0-dev
```

it is assumed, that rust is already installed.


## Building, running and installing with meson and ninja

```bash
meson setup build
cd build
ninja
```

run with
```bash
./src/debug/buoy
```

install with the following command
```bash
ninja install
```

## Building with flatpak

install dependencies
```bash
sudo apt install python3-aiohttp python3-toml
wget https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
```

building the project

```bash
python3 ./flatpak-cargo-generator.py Cargo.lock -o cargo.sources.json
flatpak --user install -y --noninteractive io.elementary.Platform/x86_64/7.3
```

## Screenshot

![Screenshot of buoy](Screenshot.png)
