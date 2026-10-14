<div align="center">

# zWipeRoll

**Swipe through. Wipe what you don’t need.**

Ultra-fast, native Windows photo declutter & gallery cleaner built with 100% pure Rust. Zero web bloat, zero Electron, zero Tauri.

![zWipeRoll Logo](assets/logo.png)

</div>

---

## ⚡ Perché zWipeRoll?

Windows Foto e l'Esplora Risorse sono notoriamente lenti quando si tratta di fare decluttering e pulizia di migliaia di immagini. **zWipeRoll** è un **decision engine** ad altissime prestazioni per il filesystem locale:

- **100% Pure Rust**: Costruito su `eframe 0.36.1` + `glow` (OpenGL). Nessun runtime Webview, nessun framework JS, zero debito tecnico.
- **Avvio Istantaneo**: Singolo eseguibile `.exe` portatile e avvio in <30ms con soli ~30-50MB di RAM.
- **Viewport-Aware Downscaling**: Le foto (anche da 50+ Megapixel) vengono scalate alla risoluzione dello schermo durante la decodifica worker pool, evitando di saturare la memoria.
- **Decoupling Filesystem / Decisioni**: 
  - `Conserva` non tocca il disco (il file è già dov'è).
  - `Cestina` segna solo una flag in memoria.
  - `Ctrl+Z` / `Undo` è istantaneo a costo I/O zero.
  - Solo nella schermata di **Riepilogo finale** i file vengono inviati in sicurezza nel **Cestino di Windows** (`trash` crate).

---

## 🎮 Controlli & Scorciatoie

| Tasto / Controllo | Azione | Note |
| :--- | :--- | :--- |
| **`D`** (o Swipe Destro) | ⭐ **Conserva** | Segna come conservata e passa alla foto successiva |
| **`A`** (o Swipe Sinistro) | 🗑️ **Cestina** | Segna per il Cestino e passa alla foto successiva |
| **Cerchio Verde** (Click) | ⭐ **Conserva** | Pulsante rotondo verde su schermo |
| **Cerchio Rosso** (Click) | 🗑️ **Cestina** | Pulsante rotondo rosso su schermo |
| **Frecce `←` / `→`** | 🔍 **Scorri** | **Solo scorrimento** avanti/indietro (nessuna decisione presa) |
| **`Ctrl + Z`** / **`U`** | ↩ **Undo** | Annulla l'ultima decisione e torna alla foto precedente |
| **`Esc`** | 📁 **Esci** | Torna alla selezione cartelle o alla dashboard mensile |

---

## 🚀 Due Modalità di Selezione

1. **📁 Cartella Specifica**: Scegli manualmente una cartella singola da ripulire.
2. **Global Scan**: Select detected Windows drives or add a custom location. The scan indexes folders containing supported images and shows progress, exclusions and read errors. Cancellation retains the previous catalog. Resynchronization preserves unselected locations and unreadable folders, together with album progress. This is not a guarantee that protected, unsupported or cloud-only images are accessible.

The exclusion and format policy is shown before scanning; its implementation lives in [the scanner](src/filesystem/scanner.rs). Global indexing does not delete source images. See [the roadmap](ROADMAP.md) for verification status and remaining work.

Windows libraries remember volume identity rather than assuming a drive letter always belongs to the same disk. Offline albums keep their progress, and a reused mount path is blocked to avoid mixing different disks. Reconnect the original volume and use **Refresh drives** when a location is unavailable. Migration limitations and hardware acceptance status are recorded in [the roadmap](ROADMAP.md#2-volume-identity-and-offline-libraries--implemented-hardware-acceptance-pending).

Real folders and virtual month albums retain decisions and their own current photo across reopening and resynchronization. **Continue last album** prefers the last selected month when applicable. Resuming follows the saved photo path; if that photo was removed, it falls back to the saved position within the remaining album. Overlapping folders share decisions and do not duplicate photos in virtual months. Real folders can be sorted by date or size in either direction; months are chronological in either direction and use local file modification dates, not EXIF capture dates. Undated photos remain in real folders.

Borderless window gestures delegate to Windows so native resizing and restoration share one owner. Drag the reserved top strip or logo area; use the green circle or double-click a title area to toggle maximize. Automated checks and outstanding native acceptance are recorded in [the roadmap](ROADMAP.md#borderless-window-controls--automated-checks-passed-native-acceptance-pending).

---

## 🎨 Brand Design & Palette

- **Primary Dark**: `#132230`
- **Background Dark**: `#0F1C27`
- **Surface Dark**: `#182936`
- **Azure (CTA)**: `#0CA5D6`
- **Mint (Accent)**: `#3ACFBF`
- **Keep**: `#22C55E`
- **Danger / Trash**: `#EF4444`
- **Typography**: Manrope (Headings / Logo) + Inter (UI)

---

## 🛠️ Build e Installazione

Requisiti: Rust toolchain installata (`rustup`).

```bash
# Compilazione debug
cargo build

# Compilazione release ottimizzata
cargo build --release
```

L'eseguibile standalone viene generato in `target/release/zwiperoll.exe`.
