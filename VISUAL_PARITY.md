# Visual parity acceptance

The approved targets are the supplied light and dark montages, not previous application screenshots. The complete horizontal logo and additional Home / Global Scan / Library / Single Folder entries are explicit user choices and remain intentional deviations from the montage content.

## Reference landmarks

Measured from the supplied PNGs before changing the shared palette. RGB samples describe individual reference pixels, not proof that a whole surface is uniformly colored.

| Landmark | Light reference | Dark reference | Candidate evidence / status |
| --- | --- | --- | --- |
| Main background at montage (200,120) | RGB 245,248,251 | RGB 19,23,29 | Palette updated; only light Home captured live |
| Sidebar at montage (80,300) | RGB 234,238,243 | RGB 23,28,34 | Palette updated; full theme/state comparison pending |
| Scan card at montage (700,180) | RGB 241,244,247 | RGB 26,31,37 | Dark surface sampled; gradients and light card variation remain unmatched |
| Active sidebar | Pale blue with blue foreground | Deep blue with light foreground | Light Home has the correct contrast direction; exact pixels unverified |
| Sidebar width / first panel width | Approximately 109/490 | Approximately 106/490 | Updated to 21.6% with responsive limits; post-change capture pending |
| Review actions | White trash and check inside red/green circles | Same icon roles | Vector icons implemented; live review capture pending |

The montage's logical scale and DPR are unknown. Compare normalized landmarks and matching screen states before claiming exact distances; do not treat a full-montage pixel difference as an application parity score.

## Open acceptance evidence

- Capture matching light and dark states for Home, scan, folder selection, library, review, trash and settings at native scale, with detail crops for icons and controls.
- Resolve sidebar proportions, typography, card/bar geometry, gradients and spacing against the reference; existing Settings and scan layouts are not accepted as identical to the montage.
- Complete neutral screenshot critique and resolve findings. No pixel-perfect acceptance has been granted.
- The current Computer Use click attempt was blocked by concurrent user input. Do not substitute offline tests or code inspection for screenshots of the missing states.
- Independent screenshot critique could not capture the candidate because the window was minimized. It measured reference sidebar widths around 22.2% (light) and 21.8% (dark), and logo widths around 75–80% of the sidebar. The user explicitly deferred visual testing; no further UI automation is authorized in this pass.

The photo renderer still uses an untinted white texture multiplier; UI theme colors must never be applied as a photo filter. The theme regression check covers active styling while switching Dark / Light / Dark, not visual parity.
