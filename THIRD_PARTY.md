# Third-party material

## Breeze icons

`src/assets/icons/breeze/` and `src/assets/icons/breeze-dark/` hold unmodified SVG files copied
from KDE's [breeze-icons](https://invent.kde.org/frameworks/breeze-icons) (version 6.31.0, as
installed under `/usr/share/icons/breeze` and `/usr/share/icons/breeze-dark`). Only the icons
the app uses are included. Symbolic links in the theme were resolved by copying the target file
under the name the app asks for; the contents are unchanged.

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
