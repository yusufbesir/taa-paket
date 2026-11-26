// Bütün müşteri takip testlerini sırayla çalıştırır: node bulut/musteri-takip/test/hepsi.mjs
// Yalnız sahte veri; depoya dosya yazmaz. Ağ ya da tarayıcı gerekmez (olc/ekran görüntüsü betikleri ayrı).
for (const f of ["dene.mjs", "dene2.mjs", "dene3.mjs", "dene4.mjs", "dene5.mjs", "dene6.mjs", "dene7.mjs"]) await import(new URL(f, import.meta.url));
console.log("müşteri takip: bütün testler geçti");
