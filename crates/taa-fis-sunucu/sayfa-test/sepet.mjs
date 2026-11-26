// Teslim sepeti sayfa testi (Playwright, gerçek tarayıcı). Sahte veri; sunucu yerine sahte `api`.
// Çalıştırma: node crates/taa-fis-sunucu/sayfa-test/sepet.mjs
// Sınar: (1) art arda iki hızlı okutmada (ilk yanıt geç gelir) iki fiş de sepette kalır (istek sırası);
// (2) bekleyen ekleme sırasında "Çıkar" listeyi bozmaz; (3) qr_kapali bir kez sorulur, sonraki pencerede karekod
// seçeneği kapalıdır; (4) dar ekranda hücre başlıkları (data-etiket) ve tablo rolleri; (5) koyu temada okunur.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const burasi = dirname(fileURLToPath(import.meta.url));
const sayfa = join(burasi, "..", "src", "sayfa");
let pw;
try {
  pw = await import("playwright");
} catch {
  const req = createRequire(join(dirname(process.execPath), "..", "lib", "node_modules", "x.js"));
  pw = req("playwright");
}
const { chromium } = pw.default || pw;

const tema = readFileSync(join(sayfa, "tema.css"), "utf8");
const ortak = readFileSync(join(sayfa, "ortak.js"), "utf8");
// Telefon sayfasının kendi stilleri (tema jetonları yerleştirilmiş): pencere ve düğmeler gerçek sayfadaki gibi görünsün.
const telefon = readFileSync(join(sayfa, "telefon.html"), "utf8");
const stil = (telefon.match(/<style>([\s\S]*?)<\/style>/) || ["", ""])[1].replace("/*@TEMA@*/", tema);
const html = `<!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<style>${stil.includes("--tk-") ? stil : tema}</style><script>${ortak}</script></head><body></body></html>`;

let tarayici;
try {
  tarayici = await chromium.launch();
} catch {
  tarayici = await chromium.launch({ executablePath: "/usr/bin/google-chrome" });
}
let hata = 0;
function dogrula(kosul, ileti) {
  if (kosul) console.log("tamam  " + ileti);
  else {
    hata++;
    console.log("HATA   " + ileti);
  }
}

async function kur(genislik) {
  const s = await tarayici.newPage({ viewport: { width: genislik, height: 900 } });
  s.on("pageerror", (e) => {
    hata++;
    console.log("HATA   sayfa hatası: " + e.message);
  });
  await s.setContent(html);
  await s.evaluate(() => {
    // Sahte sunucu: denetle istemcinin listesini geri verir, `ekle` metnindeki fişi ekler. İlk istek geç döner.
    const db = {
      "F-20261126-001": { musteri: "Deneme Bir", urun_sayisi: 1 },
      "F-20261126-002": { musteri: "Deneme İki", urun_sayisi: 2 },
      "F-20261126-003": { musteri: "Deneme Üç", urun_sayisi: 1 },
    };
    window.SAYAC = { denetle: 0, onayAc: 0 };
    const gecikmeler = [400, 20, 20, 20, 20, 20, 20, 20];
    window.API = (yontem, yol, govde) => {
      if (yol === "/api/teslim/sepet/onay-ac") {
        window.SAYAC.onayAc++;
        return Promise.reject({ kod: 409, veri: { hata: "qr_kapali", ileti: "Karekodla onay kapalı (deneme)." } });
      }
      if (yol !== "/api/teslim/sepet/denetle") return Promise.reject({ kod: 404, veri: {} });
      const n = window.SAYAC.denetle++;
      const satirlar = [];
      const ekle = (no, okutuldu) => {
        if (!db[no] || satirlar.some((x) => x.no === no)) return false;
        satirlar.push({ no, musteri: db[no].musteri, urun_sayisi: db[no].urun_sayisi, durum: "hazir", hazir: true, okutuldu, kanit: okutuldu ? "fis-karekodu" : "elle" });
        return true;
      };
      (govde.fisler || []).forEach((x) => ekle(x.no, !!x.okutuldu));
      const eklenen = [];
      if (govde.ekle) {
        const m = String(govde.ekle).match(/F-\d{8}-\d{3}/);
        if (m && ekle(m[0], /&k=/.test(govde.ekle))) eklenen.push(m[0]);
      }
      const yanit = { fisler: satirlar, eklenen, toplam_urun: satirlar.reduce((t, x) => t + x.urun_sayisi, 0), son4_gruplari: [], uyarilar: [], en_cok: 30, dolu: false };
      return new Promise((r) => setTimeout(() => r(JSON.parse(JSON.stringify(yanit))), gecikmeler[n] || 20));
    };
    window.ac = () => {
      window.SONUC = null;
      window.TKD.teslimSor({ fis: {}, oturum: { yetkiler: ["fissiz-teslim"] }, api: window.API }).then((r) => (window.SONUC = r || "kapandi"));
    };
  });
  return s;
}

async function bosta(s) {
  // Kuyruk boşalana kadar (Teslim düğmesi metni ve son denetle sayacı değişmez olana dek) bekler.
  await s.waitForTimeout(900);
}
function nolar(s) {
  return s.$$eval(".tk-sepet-tablo tbody tr td:nth-child(2)", (l) => l.map((x) => x.textContent.trim()));
}

// (1) art arda iki hızlı okutma
{
  const s = await kur(1100);
  await s.evaluate(() => window.ac());
  await s.evaluate(() => {
    const t = window.TKD.sepetAcik();
    t.ekle("http://192.168.1.20:8787/telefon#fis=F-20261126-001&k=abcdefghijkm");
    t.ekle("F-20261126-002");
  });
  await bosta(s);
  const l = await nolar(s);
  dogrula(l.includes("F-20261126-001") && l.includes("F-20261126-002"), "hızlı art arda iki okutma: iki fiş de sepette (" + l.join(", ") + ")");
  // (2) bekleyen ekleme sırasında çıkar
  await s.evaluate(() => window.TKD.sepetAcik().ekle("F-20261126-003"));
  await s.click('button[aria-label="F-20261126-001 fişini sepetten çıkar"]');
  await bosta(s);
  const l2 = await nolar(s);
  dogrula(!l2.includes("F-20261126-001") && l2.includes("F-20261126-003") && l2.includes("F-20261126-002"), "bekleyen ekleme sırasında çıkar: liste doğru (" + l2.join(", ") + ")");
  // (3) qr_kapali bir kez
  const k1 = await s.evaluate(() => window.SAYAC.onayAc);
  dogrula(k1 === 1, "qr_kapali: onay-ac bir kez çağrıldı (" + k1 + ")");
  await s.keyboard.press("Escape");
  await s.waitForTimeout(100);
  await s.evaluate(() => window.ac());
  await s.evaluate(() => window.TKD.sepetAcik().ekle("F-20261126-002"));
  await bosta(s);
  const k2 = await s.evaluate(() => window.SAYAC.onayAc);
  const qrKapali = await s.$eval('dialog input[type=radio][value=qr]', (x) => x.disabled && !x.checked);
  dogrula(k2 === 1 && qrKapali, "ikinci pencerede karekod seçeneği kapalı, yeniden sorulmadı (" + k2 + ")");
  // seçim kutusu dokunma hedefi
  const boy = await s.$eval(".tk-sepet-sec", (x) => { const r = x.getBoundingClientRect(); return Math.min(r.width, r.height); });
  dogrula(boy >= 44, "seçim kutusu dokunma hedefi " + Math.round(boy) + " px");
  await s.close();
}

// (4) dar ekran + (5) koyu tema
for (const [tema, ton] of [["kocan", "acik"], ["tebesir", "koyu"], ["civit", "koyu"]]) {
  const s = await kur(380);
  await s.evaluate(([t, o]) => { document.documentElement.dataset.tema = t; document.documentElement.dataset.ton = o; }, [tema, ton]);
  await s.evaluate(() => window.ac());
  await s.evaluate(() => window.TKD.sepetAcik().ekle("F-20261126-002"));
  await bosta(s);
  const r = await s.evaluate(() => {
    const td = document.querySelector('.tk-sepet-tablo tbody td[data-etiket="Müşteri"]');
    const once = td ? getComputedStyle(td, "::before").content : "";
    const tr = document.querySelector(".tk-sepet-tablo tbody tr");
    const fg = getComputedStyle(td).color, bg = getComputedStyle(document.querySelector("dialog")).backgroundColor;
    return { once, rol: tr && tr.getAttribute("role"), hucre: td && td.getAttribute("role"), fg, bg, tasma: document.documentElement.scrollWidth > window.innerWidth + 1 || [...document.querySelectorAll("dialog form > *")].some((e) => e.getBoundingClientRect().right > window.innerWidth + 1) };
  });
  dogrula(/Müşteri/.test(r.once), `${tema}: dar ekranda sütun adı hücrede (${r.once})`);
  dogrula(r.rol === "row" && r.hucre === "cell", `${tema}: tablo rolleri korunur`);
  dogrula(!r.tasma, `${tema}: yatay taşma yok`);
  console.log(`bilgi  ${tema}: yazı ${r.fg} / pencere ${r.bg}`);
  await s.screenshot({ path: join(process.env.SAYFA_TEST_CIKTI || "/tmp", `sepet-${tema}-dar.png`) });
  await s.close();
}

await tarayici.close();
console.log(hata ? `${hata} hata` : "hepsi geçti");
process.exit(hata ? 1 : 0);
