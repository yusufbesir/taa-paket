//! Gömülü sayfalar. Telefon sayfası veri taşımaz; belirteç tarayıcıda (localStorage) durur.

pub const MANIFEST: &str = r##"{"name":"TaaFiş","short_name":"TaaFiş","start_url":"/telefon","display":"standalone","background_color":"#ffffff","theme_color":"#1f3a5f"}"##;

pub const TELEFON: &str = r##"<!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="manifest" href="/manifest.json"><title>TaaFiş</title>
<style>:root{--a:#1f3a5f;--b:#fff;--m:#f2f4f7;--k:#b3261e}@media(prefers-color-scheme:dark){:root{--a:#8ab4f8;--b:#15181d;--m:#222831;--k:#f2b8b5}}
body{font:18px system-ui;margin:0;background:var(--b);color:var(--a)}main{padding:16px;max-width:560px;margin:auto}label{display:block;margin:12px 0 4px;font-weight:600}
input,select,textarea,button{font:inherit;width:100%;box-sizing:border-box;padding:12px;border:1px solid var(--a);border-radius:8px;background:var(--m);color:inherit}
button{background:var(--a);color:var(--b);font-weight:700;margin-top:14px;min-height:52px}button.ikinci{background:none;color:var(--a)}.kalem{background:var(--m);padding:8px;border-radius:8px;margin:8px 0}
.mesaj{padding:10px;border-radius:8px;margin:10px 0;background:var(--m)}.hata{color:var(--k)}.fis{border-bottom:1px solid var(--m);padding:10px 0}</style></head><body><main id="ana"></main>
<script>
const $=s=>document.querySelector(s),ana=$('#ana');let tok=localStorage.getItem('taa_tok');
const k=localStorage.getItem('taa_kuyruk');let kuyruk=k?JSON.parse(k):[];
const kurus=v=>Math.round(parseFloat(String(v||'0').replace(',','.'))*100)||0;
const tl=v=>(v/100).toFixed(2).replace('.',',')+' TL';
const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
async function api(y,yol,veri){const r=await fetch(yol,{method:y,headers:{'Authorization':'Bearer '+tok,'Content-Type':'application/json'},body:veri?JSON.stringify(veri):undefined});
 if(r.status==401){localStorage.removeItem('taa_tok');tok=null;throw new Error('Telefon artık yetkili değil, yeniden eşleştirin')}
 const j=await r.json().catch(()=>({}));if(!r.ok)throw new Error(j.hata||'Hata '+r.status);return j}
function mesaj(t,h){const e=$('#msg');if(e)e.innerHTML='<div class="mesaj '+(h?'hata':'')+'">'+esc(t)+'</div>'}
function eslestir(){const kod=new URLSearchParams(location.search).get('kod')||'';
 ana.innerHTML='<h1>Telefonu eşleştir</h1><div id="msg"></div><label>Bilgisayardaki kod</label><input id="kod" inputmode="numeric" value="'+esc(kod)+'"><label>Bu telefonun adı</label><input id="ad" placeholder="örn. Ayşe\'nin telefonu"><button id="e">Eşleştir</button>';
 $('#e').onclick=async()=>{try{const r=await fetch('/api/eslestir',{method:'POST',body:JSON.stringify({kod:$('#kod').value.trim(),ad:$('#ad').value.trim()})});const j=await r.json();if(!r.ok)throw new Error(j.hata||'Hata');
  localStorage.setItem('taa_tok',j.token);tok=j.token;history.replaceState(null,'','/telefon');ana.innerHTML='';ana.append(form())}catch(e){mesaj(e.message,1)}}}
function kalemHtml(){return '<div class="kalem"><input class="urun" list="urunler" placeholder="Ürün (pantolon, ceket…)"><input class="islem" placeholder="İşlem (paça kısalt…)"><input class="knot" placeholder="Not (2 cm…)"></div>'}
function form(){const d=document.createElement('div');const bugun=new Date().toISOString().slice(0,10);
 d.innerHTML='<h1>Yeni fiş</h1><div id="msg"></div><datalist id="urunler"><option>Pantolon<option>Ceket<option>Yelek<option>Gömlek<option>Palto<option>Etek</datalist>'+
 '<label>Müşteri adı</label><input id="mus"><label>Telefon</label><input id="tel" inputmode="tel"><div id="kalemler">'+kalemHtml()+'</div><button class="ikinci" id="ek">+ Ürün ekle</button>'+
 '<label>Ölçü notu</label><textarea id="olcu" rows="2"></textarea><label>Prova tarihi</label><input id="prova" type="date" min="'+bugun+'"><label>Teslim tarihi</label><input id="teslim" type="date" min="'+bugun+'">'+
 '<label>Ücret (TL)</label><input id="ucret" inputmode="decimal"><label>Ön ödeme (TL)</label><input id="on" inputmode="decimal"><label>Usta</label><input id="usta"><button id="kaydet">Kaydet</button><button class="ikinci" id="liste">Fişler / Yazdır</button>';
 d.querySelector('#ek').onclick=()=>d.querySelector('#kalemler').insertAdjacentHTML('beforeend',kalemHtml());
 d.querySelector('#liste').onclick=liste;
 d.querySelector('#kaydet').onclick=async()=>{const v=s=>d.querySelector(s).value.trim();
  const kalemler=[...d.querySelectorAll('.kalem')].map(e=>({urun:e.querySelector('.urun').value.trim(),islem:e.querySelector('.islem').value.trim(),not:e.querySelector('.knot').value.trim()})).filter(x=>x.urun||x.islem);
  const f={musteri:v('#mus'),telefon:v('#tel'),kalemler,olcu_notu:v('#olcu'),prova:v('#prova'),teslim:v('#teslim'),ucret_kurus:kurus(v('#ucret')),on_odeme_kurus:kurus(v('#on')),usta:v('#usta')};
  try{const j=await api('POST','/api/fis',f);mesaj('Kaydedildi: '+j.no)}catch(e){if(/Failed|fetch|Network/i.test(e.message)){kuyruk.push(f);localStorage.setItem('taa_kuyruk',JSON.stringify(kuyruk));mesaj('Ağ yok: taslak telefonda saklandı, bağlanınca gönderilecek')}else mesaj(e.message,1)}};
 return d}
async function kuyrukGonder(){if(!tok||!kuyruk.length)return;const kalan=[];for(const f of kuyruk){try{await api('POST','/api/fis',f)}catch(e){if(/Failed|fetch|Network/i.test(e.message))kalan.push(f)}}kuyruk=kalan;localStorage.setItem('taa_kuyruk',JSON.stringify(kuyruk))}
async function liste(){try{const l=await api('GET','/api/fisler');ana.innerHTML='<h1>Fişler</h1><div id="msg"></div><button class="ikinci" id="geri">← Yeni fiş</button>'+
 l.fisler.slice().reverse().map(f=>'<div class="fis"><b>'+esc(f.no)+'</b> '+esc(f.musteri)+'<br>Teslim '+esc(f.teslim)+' · '+esc(f.durum)+' · kalan '+tl(f.ucret_kurus-f.on_odeme_kurus)+
 '<br><button data-y="'+esc(f.no)+'">Yazdır</button>'+(f.durum=='bekliyor'?'<button class="ikinci" data-d="hazir" data-n="'+esc(f.no)+'">Hazır</button>':'')+(f.durum=='hazir'?'<button class="ikinci" data-d="teslim" data-n="'+esc(f.no)+'">Teslim edildi</button>':'')+'</div>').join('');
 $('#geri').onclick=()=>{ana.innerHTML='';ana.append(form())};
 ana.querySelectorAll('[data-y]').forEach(b=>b.onclick=async()=>{try{await api('POST','/api/fis/'+b.dataset.y+'/yazdir');mesaj('Yazıcıya gönderildi')}catch(e){mesaj(e.message,1)}});
 ana.querySelectorAll('[data-d]').forEach(b=>b.onclick=async()=>{try{await api('POST','/api/fis/'+b.dataset.n+'/durum',{durum:b.dataset.d});liste()}catch(e){mesaj(e.message,1)}})}catch(e){mesaj(e.message,1)}}
if(tok){ana.append(form());kuyrukGonder()}else eslestir();
</script></body></html>"##;

pub fn kacis(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Bilgisayardaki yönetim sayfası (yalnız bu bilgisayardan açılır).
pub fn yonetim(
    qr_svg: &str,
    kod: &str,
    url: &str,
    cihazlar: &[(String, String, String)],
) -> String {
    let satirlar: String = cihazlar
        .iter()
        .map(|(id, ad, tarih)| format!("<tr><td>{}</td><td>{}</td><td>{}</td><td><button onclick=\"kapat('{}')\">Kapat</button></td></tr>", kacis(id), kacis(ad), kacis(tarih), kacis(id)))
        .collect();
    format!(
        r##"<!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>TaaFiş yönetim</title>
<style>:root{{--a:#1f3a5f;--b:#fff}}@media(prefers-color-scheme:dark){{:root{{--a:#8ab4f8;--b:#15181d}}}}body{{font:18px system-ui;background:var(--b);color:var(--a);margin:24px}}svg{{background:#fff;padding:8px;border-radius:8px}}td,th{{padding:6px 12px;text-align:left}}button{{font:inherit;padding:8px 16px}}</style></head><body>
<h1>TaaFiş telefon eşleştirme</h1><p>Telefonla karekodu okutun (10 dakika geçerli, tek kullanımlık). Kod: <b>{kod}</b></p>{qr}<p><small>{url}</small></p>
<button onclick="location.reload()">Yeni kod üret</button><h2>Tanımlı telefonlar</h2><table><tr><th>Kod</th><th>Ad</th><th>Tarih</th><th></th></tr>{satirlar}</table>
<script>async function kapat(id){{if(!confirm('Bu telefon kapatılsın mı?'))return;await fetch('/api/yonetim/cihaz/'+id+'/kapat',{{method:'POST'}});location.reload()}}</script></body></html>"##,
        kod = kacis(kod),
        qr = qr_svg,
        url = kacis(url),
        satirlar = satirlar
    )
}
