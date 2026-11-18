<p align="center">
  <img src="icon.png" alt="Universal Freecam" width="256">
</p>

<h1 align="center">Universal Freecam</h1>

<p align="center">
  Kamera bebas untuk [Horizon](https://github.com/horizon-rust/horizon.git) (Minecraft Java 26.3 versi Rust).<br>
  Kamera lepas dari karakter dan terbang ke mana saja, sementara karakter tetap diam.
</p>

## Persyaratan

- Horizon launcher **0.2.73** atau lebih baru
- Instance di Nightly **0b14f8a** (18 November 2026) atau yang lebih baru. Build native butuh SDK yang dikirim
  bersama Nightly, jadi Nightly lama tidak bisa.
- Mod native dikompilasi di komputer kamu dan berjalan dengan hak akses yang sama seperti game, tanpa sandbox.
  Baca source-nya dulu kalau perlu.

## Pasang

1. Di launcher, buka **Mods**, lalu bagian *Add a native mod from GitHub*.
2. Tempel `https://github.com/Aiken1609/uni-freecam` dan klik **Add from GitHub**.
3. Pilih instance di Nightly yang sesuai, klik **Build**, dan setujui peringatan trust. Build pertama bisa lama
   karena launcher mengunduh toolchain Rust dan SDK.
4. Mainkan instance itu, masuk dunia, lalu tekan **F4**.

Alternatif lewat terminal:

```
horizon-launcher native-build . --instance "NamaInstance" --trust-native-code
```

## Kontrol

| Aksi | Tombol |
|---|---|
| Nyalakan atau matikan freecam | `F4` (bisa diganti di Controls) |
| Terbang | Kontrol gerak Minecraft kamu: maju, mundur, kiri, kanan, lompat (naik), sneak (turun), sprint (cepat) |
| Ubah kecepatan terbang | `Alt` + scroll |
| Pindah ke titik tersimpan (kalau kosong, simpan di sini) | `Alt` + `1`..`9` |
| Simpan titik di posisi sekarang | `Alt` + `Shift` + `1`..`9` |

Terbang mengikuti keybind yang kamu atur di Controls, jadi tidak ada pengaturan tombol terpisah.

Selama freecam aktif, karakter diam di tempat. Tombol gerak dan ganti perspektif diblokir, tapi serang, pakai item,
jatuhkan, tukar tangan, dan hotbar tetap bekerja. Kecepatan, pengali sprint, tembus blok, gerak halus, dan arah
terbang bisa diatur di layar settings mod.

## Batasan

- Kamera selalu menghadap ke arah pandang karakter, jadi kepala karakter ikut menoleh (server bisa melihatnya).
  Posisi tubuh tidak berubah. API mod saat ini belum punya hook untuk delta mouse mentah.
- Serangan dan pakai item dihitung dari posisi tubuh, bukan posisi kamera.
- Kecepatan dari `Alt` + scroll tidak disimpan setelah keluar dari freecam.
- Di server multiplayer, freecam bisa dianggap curang. Cek aturan servernya.
- Registry sudah punya freecam lain (`freecam`, `freecam_plus`, `fair_freecam`, `freecam_yannis`). Pasang satu saja.

## Untuk pengembang

```
horizon-mod.toml     manifest dan pin build Horizon
Cargo.toml           depends on horizon-mod-api
src/lib.rs           hook ke game
src/freecam_core.rs  logika kamera, tanpa dependensi, 15 test
assets/              file bahasa
```

Logika kamera terpisah dari hook game, jadi kalau API berubah cukup `src/lib.rs` yang disesuaikan. Launcher menolak
file `.rs` dan `.toml` di luar `src/`, jadi repo ini sengaja dibuat minimal. Untuk pindah build, ganti `horizon = "..."`
di `horizon-mod.toml` dengan commit Nightly tujuan (atau `cargo horizon-mod update`).

## Lisensi

MIT [LICENSE](https://github.com/Aiken1609/uni-freecam/blob/8c27bd33410294b6c5b63ca84b70fbb461773cdb/LICENSE). Logika terbang terinspirasi `freecam_plus` oleh umbx-dot (MIT).
