# CI iş akışı (henüz etkin değil)

`docs/ci/ci.yml` GitHub Actions iş akışıdır (fmt, clippy, test, audit; ubuntu + windows). Bu makinedeki gh belirtecinde `workflow` kapsamı olmadığı için `.github/workflows/` altına gönderilemedi. Etkinleştirmek için Yusuf GitHub arayüzünden dosyayı `.github/workflows/ci.yml` olarak ekleyebilir ya da `gh auth refresh -s workflow` ile kapsam verilir.

yazan: [Claude#taa-paket-A0 2026-11-04]
