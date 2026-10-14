# NMBS Vertrekbord — desktop-app (Rust)

Een volledig in Rust geschreven versie van de NMBS-vertrekbordsimulator, als los programma voor
**Windows, macOS en Linux**. Er is geen browser of installatie nodig: één uitvoerbaar bestand.

Gebouwd met [egui/eframe](https://github.com/emilk/egui) (tekenen met OpenGL), live gegevens via de
open [iRail API](https://docs.irail.be).

## Downloaden

Elke push bouwt de app automatisch via GitHub Actions (workflow **Desktop app**):

| Systeem | Bestand | Starten |
| --- | --- | --- |
| Windows 10/11 (64-bit) | `nmbs-vertrekbord-windows-x86_64.zip` | uitpakken, dubbelklik `nmbs-vertrekbord.exe` |
| macOS 11+ (Apple Silicon én Intel) | `nmbs-vertrekbord-macos-universal.zip` | uitpakken, rechtsklik op **NMBS Vertrekbord** → *Open* (eerste keer) |
| Linux (x86-64) | `nmbs-vertrekbord-linux-x86_64.tar.gz` | `tar xzf …` en `./nmbs-vertrekbord` |

De bestanden staan onder *Actions → Desktop app → (laatste run) → Artifacts*.
Wanneer je een tag `v…` pusht (bv. `v0.1.0`), worden ze ook als GitHub Release gepubliceerd.

De app is niet ondertekend met een betaald certificaat. Windows SmartScreen kan daarom
"Onbekende uitgever" tonen (*Meer info → Toch uitvoeren*), en macOS vraagt de eerste keer om
bevestiging via rechtsklik → *Open*.

## Functies

Alles van de webversie:

- Live klok in Belgische tijd (ook met verschuiving of als eigen simulatietijd met snelheid 0–60×).
- Treinen volledig bewerken: tijd, bestemming, via, opmerking, spoor, type, internationaal icoon,
  vertraging en verwachte tijd, status (*Komt aan*, *Beperkt*, *Afgeschaft*, eigen tekst) en spoorwijziging.
- Simulatie: treinen verdwijnen na vertrek, het bord vult zich aan, automatisch sorteren en *Komt aan*.
- **Live NMBS-gegevens** voor elk station: vertrek of aankomst, filter op sporen (perronscherm),
  via-stations, automatisch verversen.
- Talen NL/FR/DE/EN (ook wisselend), aantal rijen, logo, seconden, knipperende klok,
  scrollende teksten en alle kleuren instelbaar.
- Instellingen worden automatisch bewaard. Exporteren en importeren gebeurt in hetzelfde JSON-formaat
  als de webversie, dus je kunt configuraties tussen beide uitwisselen.

## Bediening

| Toets | Actie |
| --- | --- |
| `P` | instellingenpaneel tonen/verbergen |
| `F` / `F11` / dubbelklik | volledig scherm |
| `R` | willekeurige trein toevoegen |
| `Esc` | paneel sluiten |

Opstartopties: `--display` (zonder paneel), `--fullscreen` (volledig scherm), `--help`.
Handig voor een apart scherm: `nmbs-vertrekbord --display --fullscreen`.

De instellingen staan in:

- Windows: `%APPDATA%\nmbs-vertrekbord\config.json`
- macOS: `~/Library/Application Support/nmbs-vertrekbord/config.json`
- Linux: `~/.config/nmbs-vertrekbord/config.json`

## Zelf bouwen

Vereist: [Rust](https://rustup.rs) (stabiele versie).

```sh
cd desktop
cargo run --release
```

Het programma staat daarna in `desktop/target/release/`. Testen: `cargo test`.

Op Linux heeft de app de gewone bureaubladbibliotheken nodig (OpenGL, X11 of Wayland en
`libxkbcommon`); die zijn op elke desktopinstallatie aanwezig.

## Opbouw

| Bestand | Inhoud |
| --- | --- |
| `src/main.rs` | opstarten, venster, lettertypes |
| `src/app.rs` | toestand, live verversen, opslaan, sneltoetsen |
| `src/board.rs` | het tekenen van het bord |
| `src/panel.rs` | het instellingenpaneel |
| `src/model.rs` | gegevensmodel, simulatie en tijdrekenen |
| `src/live.rs` | iRail-client (achtergrondthread met wachtrij) |
| `src/clock.rs` | klok (Belgische tijd / simulatie) |

Het lettertype Source Sans 3 (© Adobe) is ingebouwd onder de SIL Open Font License, zie
`assets/fonts/OFL.txt`.
