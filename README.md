<p align="center">
  <img src="icon.png" alt="Universal Freecam" width="256">
</p>

<h1 align="center">Universal Freecam</h1>

<p align="center">
  Kamera bebas untuk Horizon (Minecraft Java 26.3 versi Rust).<br>
  Kamera lepas dari karakter dan terbang ke mana saja, sementara karakter tetap diam.
</p>

## Persyaratan

- Horizon Nightly **f037744** (21 November 2026), atau build lain yang API mod-nya masih cocok
- Mod ini berupa WebAssembly: berjalan di sandbox game, tanpa kompilasi di komputer pemain dan tanpa izin
  "trust native code"
- Game memeriksa `mod.wasm` saat memuatnya dan menolak kalau layout API-nya berbeda. Satu paket belum tentu
  jalan di semua Nightly (lihat Batasan)

## Pasang

1. Dapatkan `uni_freecam.hmod` (dari halaman Releases repo ini kalau ada, atau bangun sendiri, lihat bawah).
2. Di launcher, klik ⋮ pada instance, lalu **Open folder**. Salin `uni_freecam.hmod` ke folder `horizon-mods`.
3. Tambah satu baris di `horizon-mods.txt` (di sebelah `options.txt`):
   ```
   mod.uni_freecam:true
   ```
4. Jalankan game, masuk dunia, dan tekan **F4**. Kalau game menolak mod-nya, alasannya tampil merah di layar Mods.

## Bangun sendiri

Butuh [rustup](https://rustup.rs) (di Windows juga Visual Studio Build Tools dengan komponen C++).

```
cargo install --locked --git https://github.com/horizon-rust/horizon --rev f037744a79f9982c47876f49c5a3067279f88706 cargo-horizon-mod
cargo horizon-mod build --wasm
```

Hasilnya `target/horizon-mod/uni_freecam.hmod`. Pasang `cargo-horizon-mod` dari commit Nightly yang kamu pakai.
Build pertama memasang toolchain Rust yang dipin.

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

- Paket wasm tidak punya rentang versi yang dijamin. Kalau game menolaknya di Nightly lain, bangun ulang untuk
  build itu: `cargo horizon-mod update --horizon <commit>` lalu `cargo horizon-mod build --wasm`.
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

Logika kamera terpisah dari hook game, jadi kalau API berubah cukup `src/lib.rs` yang disesuaikan. Untuk pindah build, ganti `horizon = "..."`
di `horizon-mod.toml` dengan commit Nightly tujuan (atau `cargo horizon-mod update`).

## Lisensi

MIT [LICENSE](https://github.com/Aiken1609/uni-freecam/blob/8c27bd33410294b6c5b63ca84b70fbb461773cdb/LICENSE). Logika terbang terinspirasi `freecam_plus` oleh umbx-dot (MIT).
