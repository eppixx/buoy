# Updating the translations

When adding a new file that contains the `gettext` function it needs to be added to `po/POTFILES`.
All needed files can be found by replacing `po/POTFILES` with output fo the following
```bash
grep -rl "gettext(" ./src
```
Now you can update the translation files with
```bash
ninja -C build com.github.eppixx.buoy-update-po
ninja -C build extra-update-po
```
and update the translations of the po files with an application of your choosing (like [Poedit](https://flathub.org/apps/net.poedit.Poedit)).
