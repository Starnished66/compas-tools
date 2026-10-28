# Third-party material

## Breeze icons

`src/assets/icons/breeze/` and `src/assets/icons/breeze-dark/` hold unmodified SVG files copied
from KDE's [breeze-icons](https://invent.kde.org/frameworks/breeze-icons) (version 6.31.0, as
installed under `/usr/share/icons/breeze` and `/usr/share/icons/breeze-dark`). Only the icons
the app uses are included. Symbolic links in the theme were resolved by copying the target file
under the name the app asks for; the contents are unchanged.

The icons of the "Player settings" screen (`configure-symbolic`, `audio-volume-high-symbolic`,
`preferences-desktop-display-randr-symbolic`, `phone-battery-symbolic`,
`media-playback-start-symbolic`, `preferences-system-bluetooth-symbolic`,
`configure-shortcuts-symbolic`, all size 22) were added from the pinned release
`breeze-icons-6.31.0.tar.xz` (https://download.kde.org/stable/frameworks/6.31/, SHA-256
`82ab1c13f782cb37176e7c13c5fb2c77536a65f24b85d09127f3575f917fb56d`, as published by KDE).
The `breeze/` files are the tarball's SVGs with symbolic links resolved; the `breeze-dark/` files are
the colour-patched variants the same release's build installs, unmodified.

- Copyright (C) 2014 Uri Herrera and others.
- Licence: GNU Lesser General Public License, version 3 or (at your option) any later version.
  The licence statement shipped with the theme is in `src/assets/icons/COPYING-ICONS`.
- To replace an icon, copy the new file over the old one from both theme folders; do not edit
  the SVG.

## Noto Sans

`src/assets/fonts/NotoSans-Variable.ttf` is the unmodified variable font `NotoSans[wght].ttf`
(version 1.32) from the Noto Project, the font Breeze uses.

- Copyright 2022 The Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic).
- Licence: SIL Open Font License, Version 1.1, in `src/assets/fonts/OFL.txt`.

## Breeze colour values

The colour tokens in `src/styles/tokens.css` are the values of KDE's colour schemes "Breeze Light"
(`/usr/share/color-schemes/BreezeLight.colors`) and "Breeze Dark" (`BreezeDark.colors`), from
the `plasma-breeze` package, which carries the KDE project's LGPL licence. Each token names the
colour scheme group and key it comes from.

## compas-sdtool and the player's code

`compas-sdtool` is a C program built from `sdtool/` in this repository together with the
sources of the Compás player (`Starnished66/compas-player`) at the commit pinned as `PLAYER_REF`
in `.github/workflows/build.yml`. The player's sources, and the libraries vendored in them, carry
their own licences. Every installer holds the player's licence texts under `licenses/sdtool/`,
with a `BUILD_INFO.txt` naming the exact player commit and the commit of this repository the
tool was built from, so the corresponding sources can be fetched from GitHub.
