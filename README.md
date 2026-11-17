# Universal Freecam untuk Horizon (Minecraft 26.3 versi Rust)

Kamera lepas dari pemain: terbang bebas, badan pemain tetap di tempat. Satu inti logika
(`core/freecam_core.rs`) dipakai dua adapter, supaya cocok dengan beberapa build Horizon.

## Status (jujur)
- Inti logika: 15 unit test lulus.
- Adapter API baru (`native/`): `cargo check` bersih terhadap `horizon-mod-api` build `87601bd` (main, 17 Nov 2026).
- Adapter build lama (`legacy/`): bundle horizon-mixins diterapkan ke build `9f42064` (Nightly 13 Nov 2026) dan `cargo check -p horizon-client` lulus.
- **Belum pernah dijalankan di dalam game.** Aku tidak bisa menjalankan Horizon di sandbox. Anggap ini beta; laporkan bug.

## Kontrol
| Aksi | Tombol |
|---|---|
| Nyalakan/matikan freecam | F4 (bisa diganti di Controls) |
| Terbang | W A S D, Space naik, Shift turun, Ctrl cepat |
| Ubah kecepatan | Scroll mouse saat freecam aktif |
| Lompat ke titik tersimpan (kosong = simpan) | Alt + 1..9 |
| Simpan titik di posisi sekarang | Alt + Shift + 1..9 |

Pengaturan (kecepatan, pengali sprint, tembus blok, gerak halus, terbang ke arah pandang) ada di layar Mods/Settings.

## Pilih adapter
| Build Horizon kamu | Pakai |
|---|---|
| `87601bd` / main terbaru (punya `mod_api::Hooks`) | `native/` → `build/uni_freecam-0.1.0.hmod` |
| `9f42064` (Nightly 13 Nov 2026, route mixin) | `legacy/` (bundle horizon-mixins) |

Build lain: ubah pin lalu build ulang, mis. `python tools/pack.py package-native --horizon <sha 40 hex>`.
Kalau API hook berubah, perbaikannya cukup di `native/src/lib.rs`; inti tidak ikut berubah.

## Pasang (build baru)
```sh
python tools/pack.py package-native
unzip build/uni_freecam-0.1.0.hmod -d uni_freecam_project
cd uni_freecam_project
horizon-launcher native-build . --instance "Nama Instance" --trust-native-code
```
(`native/` sendiri punya placeholder pin; selalu bangun dari hasil `package-native`.)
`--trust-native-code` wajib: mod native berjalan dengan hak pemain, tanpa sandbox.

## Pasang (build lama 9f42064)
```sh
python tools/pack.py apply --horizon <checkout horizon 9f42064> --mixins-tool <horizon-mixins> --out <folder>
python tools/pack.py check --stage <folder>
```

## Batasan
- Adapter API baru tidak punya hook delta mouse mentah: arah kamera = arah pandang pemain, jadi kepala pemain ikut berputar (server melihat kepala menoleh, posisi tetap).
- Adapter lama: Alt+angka juga ikut mengganti slot hotbar.
- Adapter API baru: kecepatan dari scroll tidak disimpan setelah keluar freecam.
- Sudah ada freecam lain di registry (freecam, freecam_plus, fair_freecam, freecam_yannis). Jangan pasang bersamaan.
- Di server multiplayer, freecam bisa dianggap curang; pakai sesuai aturan server.

Lisensi: MIT.
