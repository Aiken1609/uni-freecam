# uni-freecam

Freecam untuk Horizon (Minecraft 26.3 yang ditulis ulang dalam Rust). Kamera lepas dari pemain dan bisa
terbang ke mana saja, sementara karakter kamu diam di tempat.

Ini mod native: kodenya Rust, dikompilasi launcher, dan jalan dengan hak akses yang sama seperti game.
Baca dulu sebelum dipasang. Sampai sekarang belum ada yang menjalankannya di dalam game, jadi anggap
ini versi awal.

## Pasang

Di launcher, buka halaman Mods, bagian "Add a native mod from GitHub", tempel:

```
https://github.com/Aiken1609/uni-freecam
```

Setujui peringatan trust, lalu Build. Build pertama bisa lama karena launcher mengunduh Rust dan SDK game.

Lewat terminal:

```
horizon-launcher mods add-github https://github.com/Aiken1609/uni-freecam --trust-native-code
```

Atau dari clone repo ini:

```
horizon-launcher native-build . --instance "NamaInstance" --trust-native-code
```

Mod ini baru bisa dibangun di build Horizon yang sudah punya API mod bertipe (`mod_api::Hooks`). Pin build ada
di `horizon-mod.toml` (sekarang commit `87601bd`, main 17 November 2026). Kalau instance kamu masih di Nightly
yang lebih tua dan build gagal di tahap SDK, update Horizon dulu, atau pindahkan pin dengan
`cargo horizon-mod update`.

## Kontrol

- **F4**: nyalakan atau matikan freecam (bisa diganti di Controls)
- Terbang pakai kontrol Minecraft kamu sendiri: maju, mundur, kiri, kanan, lompat (naik), sneak (turun), dan
  sprint (lebih cepat), seperti di spectator. Kalau kamu ubah keybind-nya di Controls, freecam ikut berubah, tidak
  ada pengaturan tombol tambahan. Bawaan: **W A S D**, **Space**, **Shift**, **Ctrl**.
- **Scroll**: ubah kecepatan terbang selama freecam aktif
- **Alt + 1..9**: pindah ke titik yang tersimpan; kalau slotnya kosong, posisi sekarang disimpan di situ
- **Alt + Shift + 1..9**: simpan posisi sekarang ke slot itu

Selama freecam aktif, serang, pakai item, jatuhkan item, tukar tangan, dan tombol hotbar diblokir supaya karakter tidak ikut
bergerak atau beraksi. Kecepatan, pengali sprint, tembus blok, gerak halus, dan arah terbang bisa diatur di
layar settings mod.

## Batasan

- API mod saat ini tidak punya hook untuk delta mouse mentah. Kamera selalu menghadap ke arah pandang karakter,
  jadi kepala karakter ikut menoleh (server bisa melihatnya). Posisi tubuh tidak berubah.
- Kecepatan hasil scroll tidak disimpan setelah keluar dari freecam.
- Di server multiplayer, freecam bisa dianggap curang. Cek aturan servernya.
- Registry sudah punya freecam lain (`freecam`, `freecam_plus`, `fair_freecam`, `freecam_yannis`). Pasang
  satu saja.

## Struktur repo

```
horizon-mod.toml     manifest dan pin build Horizon
Cargo.toml           depends on horizon-mod-api (diambil launcher ke .horizon/sdk/)
src/lib.rs           hook ke game
src/freecam_core.rs  logika kamera, tanpa dependensi, ada 15 test
core-tests/          crate kecil untuk menjalankan test inti: cd core-tests && cargo test
legacy/              bundle horizon-mixins untuk Nightly 9f42064 (lihat bawah)
tools/pack.py        pembuat paket .hmod / .hnmod
compat.json          build mana yang pernah dicek
```

Logika kamera ada di satu file dan tidak tahu apa-apa soal game, jadi kalau API hook berubah, yang diubah cukup
`src/lib.rs`.

## Build lama (Nightly 9f42064)

Launcher terbaru menolak format lama (`horizon-mixin.toml`, `horizon-native.json`), jadi folder `legacy/` hanya
berguna kalau kamu masih memakai launcher dan game yang lama:

```
python tools/pack.py package-legacy        # build/uni_freecam-0.1.0.hnmod
```

Bundle ini sudah dicek terhadap source `9f42064` (`cargo check -p horizon-client` lulus), tapi tidak bisa
dipakai bersama launcher yang memakai format baru.

## Bikin paket untuk build lain

```
python tools/pack.py package-native --horizon <sha 40 karakter hex>
```

Hasilnya `build/uni_freecam-0.1.0.hmod`; pin hanya diganti di paket, bukan di repo.

Lisensi MIT. Logika terbang terinspirasi `freecam_plus` (umbx-dot, MIT).
