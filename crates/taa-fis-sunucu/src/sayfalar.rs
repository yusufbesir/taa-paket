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
let sonNo=null;let kat={islemler:[]};
const GRUPLAR=['Pantolon','Ceket','Gömlek','Yelek','Takım'];
function kalemHtml(){const d=document.createElement('div');d.className='kalem';
 d.innerHTML='<select class="grup"><option value="">Ürün seçin</option>'+GRUPLAR.map(g=>'<option>'+g+'</option>').join('')+'<option value="*">Diğer (yazarak)</option></select><div class="icerik"></div>';
 const ic=d.querySelector('.icerik');
 d.querySelector('.grup').onchange=e=>{const g=e.target.value;ic.innerHTML='';if(!g)return;
  if(g==='*'){ic.innerHTML='<input class="urun" placeholder="Ürün"><input class="islem" placeholder="İşlem"><input class="knot" placeholder="Not">';return}
  const l=kat.islemler.filter(i=>i.grup===g);
  ic.innerHTML='<select class="kod"><option value="">İşlem seçin</option>'+l.map(i=>'<option value="'+esc(i.kod)+'">'+esc(i.ad)+'</option>').join('')+'</select><div class="det"></div>';
  ic.querySelector('.kod').onchange=ev=>{const i=kat.islemler.find(x=>x.kod===ev.target.value),det=ic.querySelector('.det');det.innerHTML='';if(!i)return;
   if(i.varyantlar&&i.varyantlar.length)det.innerHTML+='<select class="varyant"><option value="">Seçenek seçin</option>'+i.varyantlar.map(v=>'<option>'+esc(v)+'</option>').join('')+'</select>';
   if(i.olcu)det.innerHTML+='<input class="olcu" inputmode="decimal" placeholder="Ölçü (cm, biliniyorsa)"><label class="onay"><input type="checkbox" class="igneli" style="width:auto"> İğneyle işaretlendi (cm yok)</label><input class="pay" inputmode="decimal" placeholder="İçeride kalan pay (cm)">';
   det.innerHTML+='<input class="knot" placeholder="Not">';
   if(i.sinir_mm){det.innerHTML+='<label class="onay" hidden><input type="checkbox" class="usta" style="width:auto"> Usta onayladı (sınır '+(i.sinir_mm/10)+' cm)</label>';
    det.querySelector('.olcu').oninput=o=>{det.querySelector('.onay').hidden=!(kurus(o.target.value)>i.sinir_mm*10)}}}};
 return d}
function kalemOku(e){const g=e.querySelector('.grup').value;if(!g)return null;
 const mm=v=>{const x=kurus(v);return x?Math.round(x/10):null};
 if(g==='*')return{urun:e.querySelector('.urun').value.trim(),islem:e.querySelector('.islem').value.trim(),not:e.querySelector('.knot').value.trim()};
 const kod=e.querySelector('.kod').value;const i=kat.islemler.find(x=>x.kod===kod);if(!i)return null;
 const q=s=>e.querySelector(s);return{urun:g,islem:i.ad,islem_kodu:kod,not:q('.knot').value.trim(),varyant:q('.varyant')?q('.varyant').value||null:null,
  olcu_mm:q('.olcu')?mm(q('.olcu').value):null,pay_mm:q('.pay')?mm(q('.pay').value):null,usta_onayli:!!(q('.usta')&&q('.usta').checked),igneli:!!(q('.igneli')&&q('.igneli').checked)}}
function form(){const d=document.createElement('div');const bugun=new Date().toISOString().slice(0,10);
 d.innerHTML='<h1>Yeni tadilat fişi</h1><div id="msg"></div>'+
 '<label>Satış danışmanı</label><input id="sat"><label>Ölçü alan</label><input id="olcualan">'+
 '<label>Müşteri adı soyadı</label><input id="mus"><label>Telefon</label><input id="tel" inputmode="tel"><div id="kalemler"></div><button class="ikinci" id="ek">+ Ürün / işlem ekle</button>'+
 '<label>Ölçü notu</label><textarea id="olcu" rows="2"></textarea><label>Prova tarihi (isteğe bağlı)</label><input id="prova" type="date" min="'+bugun+'"><label>Teslim tarihi</label><input id="teslim" type="date" min="'+bugun+'"><label>Teslim saati</label><input id="saat" type="time" value="17:00">'+
 '<label>Ücret (TL)</label><input id="ucret" inputmode="decimal"><label>Ön ödeme (TL)</label><input id="on" inputmode="decimal"><label>Usta</label><input id="usta"><label><input type="checkbox" id="kargo"> Kargo yapılacak</label><input id="kargoad" placeholder="Alıcı ad soyad"><textarea id="kargoadres" rows="2" placeholder="Açık adres (mahalle, sokak, no, ilçe/il)"></textarea><input id="kargotel" inputmode="tel" placeholder="Alıcı telefonu"><input id="kargonot" placeholder="Kargo notu (isteğe bağlı)">'+'<button id="kaydet">Kaydet</button><button class="ikinci" id="foto">📷 Son fişe iğneli ürün fotoğrafı ekle</button><button class="ikinci" id="liste">Fişler / Yazdır</button>';
 const v=s=>d.querySelector(s).value.trim();
 d.querySelector('#sat').value=localStorage.getItem('taa_sat')||'';d.querySelector('#olcualan').value=localStorage.getItem('taa_olcualan')||'';
 const kl=d.querySelector('#kalemler');kl.append(kalemHtml());
 d.querySelector('#ek').onclick=()=>kl.append(kalemHtml());
 api('GET','/api/islemler').then(j=>{kat=j}).catch(e=>mesaj(e.message,1));
 d.querySelector('#liste').onclick=liste;d.querySelector('#foto').onclick=()=>{if(sonNo)fotoSec(sonNo);else mesaj('Önce fişi kaydedin',1)};
 d.querySelector('#kaydet').onclick=async()=>{
  const kalemler=[...d.querySelectorAll('.kalem')].map(kalemOku).filter(Boolean);
  const f={satis_danismani:v('#sat'),olcu_alan:v('#olcualan'),musteri:v('#mus'),telefon:v('#tel'),kalemler,olcu_notu:v('#olcu'),prova:v('#prova'),teslim:v('#teslim'),teslim_saati:v('#saat')||'17:00',ucret_kurus:kurus(v('#ucret')),on_odeme_kurus:kurus(v('#on')),usta:v('#usta'),kargo:d.querySelector('#kargo').checked,kargo_notu:v('#kargonot'),kargo_ad:v('#kargoad'),kargo_adres:v('#kargoadres'),kargo_telefon:v('#kargotel')};
  localStorage.setItem('taa_sat',f.satis_danismani);localStorage.setItem('taa_olcualan',f.olcu_alan);
  try{const j=await api('POST','/api/fis',f);sonNo=j.no;mesaj('Kaydedildi: '+j.no+((j.uyarilar||[]).length?' — '+j.uyarilar.join(' | '):''))}catch(e){if(/Failed|fetch|Network/i.test(e.message)){kuyruk.push(f);localStorage.setItem('taa_kuyruk',JSON.stringify(kuyruk));mesaj('Ağ yok: taslak telefonda saklandı, bağlanınca gönderilecek')}else mesaj(e.message,1)}};
 return d}
async function fotoYukle(no,dosya){const b=await createImageBitmap(dosya);const o=Math.min(1,1280/Math.max(b.width,b.height));const c=document.createElement('canvas');c.width=Math.round(b.width*o);c.height=Math.round(b.height*o);c.getContext('2d').drawImage(b,0,0,c.width,c.height);
 const blob=await new Promise(r=>c.toBlob(r,'image/jpeg',0.8));const r=await fetch('/api/fis/'+no+'/foto',{method:'POST',headers:{'Authorization':'Bearer '+tok,'Content-Type':'image/jpeg'},body:blob});const j=await r.json().catch(()=>({}));if(!r.ok)throw new Error(j.hata||'Hata '+r.status);return j}
function fotoSec(no){const i=document.createElement('input');i.type='file';i.accept='image/*';i.setAttribute('capture','environment');i.onchange=async()=>{if(!i.files[0])return;try{await fotoYukle(no,i.files[0]);mesaj('Fotoğraf eklendi: '+no)}catch(e){mesaj(e.message,1)}};i.click()}
async function kuyrukGonder(){if(!tok||!kuyruk.length)return;const kalan=[];for(const f of kuyruk){try{await api('POST','/api/fis',f)}catch(e){if(/Failed|fetch|Network/i.test(e.message))kalan.push(f)}}kuyruk=kalan;localStorage.setItem('taa_kuyruk',JSON.stringify(kuyruk))}
async function liste(){try{const l=await api('GET','/api/fisler');ana.innerHTML='<h1>Fişler</h1><div id="msg"></div><button class="ikinci" id="geri">← Yeni fiş</button>'+
 l.fisler.slice().reverse().map(f=>'<div class="fis"><b>'+esc(f.no)+'</b> '+esc(f.musteri)+'<br>Teslim '+esc(f.teslim)+' '+esc(f.teslim_saati||'')+((f.teslim_degisiklikleri||[]).length?' (değişti: '+f.teslim_degisiklikleri.length+')':'')+' · '+esc(f.durum)+' · kalan '+tl(f.ucret_kurus-f.on_odeme_kurus)+
 '<br><button data-y="'+esc(f.no)+'">Yazdır</button><button class="ikinci" data-f="'+esc(f.no)+'">📷 Foto</button>'+(f.durum=='bekliyor'?'<button class="ikinci" data-d="terzihanede" data-n="'+esc(f.no)+'">Terzihaneye verildi</button>':'')+(f.durum=='terzihanede'?'<button class="ikinci" data-d="hazir" data-n="'+esc(f.no)+'">Hazır</button>':'')+(['bekliyor','terzihanede','hazir'].includes(f.durum)?'<button class="ikinci" data-t="'+esc(f.no)+'">Teslim tarihini değiştir</button>':'')+(f.durum=='hazir'?'<button class="ikinci" data-d="teslim" data-n="'+esc(f.no)+'">Teslim edildi</button>':'')+'</div>').join('');
 $('#geri').onclick=()=>{ana.innerHTML='';ana.append(form())};
 ana.querySelectorAll('[data-f]').forEach(b=>b.onclick=()=>fotoSec(b.dataset.f));ana.querySelectorAll('[data-y]').forEach(b=>b.onclick=async()=>{try{await api('POST','/api/fis/'+b.dataset.y+'/yazdir');mesaj('Yazıcıya gönderildi')}catch(e){mesaj(e.message,1)}});
 ana.querySelectorAll('[data-t]').forEach(b=>b.onclick=async()=>{const y=prompt('Yeni teslim tarihi (YYYY-AA-GG)');if(!y)return;const sa=prompt('Yeni teslim saati (SS:DD)','17:00')||'17:00';const n=prompt('Neden değişti?');if(!n)return;try{await api('POST','/api/fis/'+b.dataset.t+'/teslim',{yeni:y,saat:sa,neden:n});liste()}catch(e){mesaj(e.message,1)}})
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
