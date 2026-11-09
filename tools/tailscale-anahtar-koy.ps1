# Sahip: Yusuf. yazan: [Claude#taa-paket-harici 2026-11-09]
# Bu PC'de Tailscale'i otomatik kurup ağa katar: auth key'i <kök>\gizli\tailscale.anahtar dosyasına yazar,
# sonra `taa-kur harici tailscale` çalıştırır (kurulu değilse indirir, SHA256+imza doğrular, sessiz kurar, `tailscale up` yapar).
# Anahtar Tailscale yönetim konsolundan üretilir: önceden yetkili (pre-approved), etiketli (tag:taa), süreli, tek kullanımlık önerilir.
# Kullanım (yönetici PowerShell):
#   .\tailscale-anahtar-koy.ps1                       # anahtar sorulur (ekranda görünmez)
#   $env:TAA_TAILSCALE_ANAHTAR = "tskey-auth-..."; .\tailscale-anahtar-koy.ps1
# Ortam/ayar: kök TAA_KOK ya da D:\Programlar\TAA. Anahtar günlüğe/ekrana yazılmaz. İzin (ACL) değiştirilmez.
param([string]$Kok = $(if ($env:TAA_KOK) { $env:TAA_KOK } else { 'D:\Programlar\TAA' }))
$ErrorActionPreference = 'Stop'

$kur = Join-Path $Kok 'taa-kur.exe'
if (-not (Test-Path -LiteralPath $kur)) { throw "taa-kur.exe bulunamadı: $kur. Önce TAA paketini kurun." }

if ($env:TAA_TAILSCALE_ANAHTAR) {
    $anahtar = $env:TAA_TAILSCALE_ANAHTAR.Trim()
} else {
    $guvenli = Read-Host -AsSecureString 'Tailscale auth key (tskey-auth-...)'
    $anahtar = [System.Net.NetworkCredential]::new('', $guvenli).Password.Trim()
}
if ($anahtar -notmatch '^tskey-[A-Za-z0-9_-]+$') { throw 'Anahtar tskey- ile başlamalı ve yalnız harf, rakam, - ve _ içermeli.' }

$gizli = Join-Path $Kok 'gizli'
New-Item -ItemType Directory -Force -Path $gizli | Out-Null
$dosya = Join-Path $gizli 'tailscale.anahtar'
if (Test-Path -LiteralPath $dosya) {
    # Silme yok: eskisi yanına yedeklenir.
    Move-Item -LiteralPath $dosya -Destination ($dosya + '.eski-' + (Get-Date -Format 'yyyyMMddHHmmss'))
}
[System.IO.File]::WriteAllText($dosya, $anahtar, [System.Text.UTF8Encoding]::new($false))
$anahtar = $null
Write-Host "Anahtar yazıldı: $dosya"

& $kur harici tailscale
if ($LASTEXITCODE -ne 0) { throw "taa-kur harici $LASTEXITCODE koduyla bitti." }
Write-Host 'Tamam. Durum: tailscale status'
