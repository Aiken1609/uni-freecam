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
di `horizon-mod.toml` (sekarang commit `0b14f8a`, Nightly 18 November 2026). Kalau instance kamu masih di Nightly
yang lebih tua dan build gagal di tahap SDK, update Horizon dulu, atau pindahkan pin dengan
`cargo horizon-mod update`.

## Kontrol

- **F4**: nyalakan atau matikan freecam (bisa diganti di Controls)
- Terbang pakai kontrol Minecraft kamu sendiri: maju, mundur, kiri, kanan, lompat (naik), sneak (turun), dan
  sprint (lebih cepat), seperti di spectator. Kalau kamu ubah keybind-nya di Controls, freecam ikut berubah, tidak
  ada pengaturan tombol tambahan. Bawaan: **W A S D**, **Space**, **Shift**, **Ctrl**.
- **Alt + scroll**: ubah kecepatan terbang selama freecam aktif (scroll biasa tetap ganti slot hotbar)
- **Alt + 1..9**: pindah ke titik yang tersimpan; kalau slotnya kosong, posisi sekarang disimpan di situ
- **Alt + Shift + 1..9**: simpan posisi sekarang ke slot itu

Selama freecam aktif, karakter diam di tempat: tombol gerak (dan ganti perspektif) diblokir, tapi serang, pakai
item, jatuhkan, tukar tangan, dan hotbar tetap jalan seperti biasa. Karakter menghadap ke arah kamera, jadi
membidik tetap mengikuti crosshair, walaupun posisi kamera dan posisi karakter berbeda. Kecepatan, pengali
sprint, tembus blok, gerak halus, dan arah terbang bisa diatur di layar settings mod.

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
assets/              file bahasa
```

Launcher membaca root repo ini sebagai satu project mod dan menolak file `.rs` atau `.toml` di luar `src/`,
jadi repo sengaja dibuat minimal. Logika kamera ada di satu file dan tidak tahu apa-apa soal game; kalau API
hook berubah, yang diubah cukup `src/lib.rs`.

Test inti: setelah `cargo horizon-mod check` sekali (mengambil SDK), jalankan `cargo test`.

Untuk membuat paket `.hmod` dengan pin build lain, ubah `horizon = "..."` di `horizon-mod.toml` (atau jalankan
`cargo horizon-mod update`), lalu `cargo horizon-mod build`.

Lisensi MIT. Logika terbang terinspirasi `freecam_plus` (umbx-dot, MIT).
