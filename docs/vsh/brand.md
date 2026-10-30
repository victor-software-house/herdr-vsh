# Herdr VSH banner

| File | Use |
|:--|:--|
| [`banner.svg`](banner.svg) | README header, 1200×240, light card. |
| [`banner-dark.svg`](banner-dark.svg) | Same, dark card. |

The README pairs the two with `<picture>` and `prefers-color-scheme`.

## Mark

Herdr's own icon (`assets/icons/herdr-ui-icon-clean.svg`): the ram on a rounded tile.
Each banner carries the opposite card's mark, so the tile stands out: an ivory ram on charcoal on the light card, Herdr's charcoal ram on ivory on the dark card.
A ring in the tile colour runs inside the tile's edge, clipped to it, so the rounded square stays whole where the ram reaches the edge.

## Palette

| Role | Light | Dark |
|:--|:--|:--|
| Card | `#f4f1e9` | `#15171a` |
| Card hairline | `#ddd8cc` | `#2a2d31` |
| Wordmark | `#303438` | `#f4f1e9` |
| Tagline and command | `#6f6a60` | `#9a958b` |
| Command outline | `#6f6a60` | `#8d887e` |
| Cursor | `#d9362b` | `#ff3b30` |

The cursor is the block cursor of the `rex-static-noir` terminal theme, set after the wordmark.

## Text

Set in [Geist Mono](https://github.com/vercel/geist-font) (OFL) and converted to outlines, so nothing depends on a font at render time: wordmark Black 60px with −3 tracking, tagline Medium 17px, command Regular 13px.
To change the wording, reshape with `fonttools` and `uharfbuzz` at those sizes rather than adding a `<text>` element.
