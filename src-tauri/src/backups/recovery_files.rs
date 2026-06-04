use std::fs;
use std::path::Path;

const RECOVERY_SCRIPT_NAME: &str = "decrypt-crvi-backup.ps1";
const RECOVERY_README_NAME: &str = "README_RECOVERY.txt";

pub fn ensure_recovery_support_files(path: &Path) -> Result<(), String> {
    let readme_path = path.join(RECOVERY_README_NAME);
    let script_path = path.join(RECOVERY_SCRIPT_NAME);

    fs::write(&readme_path, recovery_readme_contents())
        .map_err(|e| format!("Impossible d'écrire le guide de recovery backup: {}", e))?;
    fs::write(&script_path, recovery_script_contents())
        .map_err(|e| format!("Impossible d'écrire le script de recovery backup: {}", e))?;
    Ok(())
}

fn recovery_readme_contents() -> &'static str {
    "Backups CRVI\n\
\n\
- `*.crvibak` : nouveau format chiffré CRVI.\n\
  - `Protégé Windows (DPAPI)` : restaurable avec le meme compte Windows sur le poste d'origine.\n\
  - `Portable chiffré (passphrase)` : restaurable avec le mot de passe saisi lors de la création.\n\
- `*.sqlite` : ancien format non chiffré conserve pour compatibilite.\n\
\n\
Recovery sans l'application:\n\
1. Ouvrir PowerShell.\n\
2. Aller dans ce dossier.\n\
3. Lancer `./decrypt-crvi-backup.ps1` pour ouvrir le mode guide.\n\
4. Le script liste les backups du dossier et propose un choix numerote.\n\
5. Pour un backup portable, le mot de passe est demande seulement si necessaire.\n\
6. Le script produit un `.sqlite` exploitable ou peut aussi restaurer vers un chemin cible.\n\
\n\
Exemples rapides:\n\
- `./decrypt-crvi-backup.ps1`\n\
- `./decrypt-crvi-backup.ps1 -ListOnly`\n\
- `./decrypt-crvi-backup.ps1 -InputPath .\\nom-du-backup.crvibak`\n\
- `./decrypt-crvi-backup.ps1 -InputPath .\\nom-du-backup.crvibak -RestoreTo C:\\donnees\\crvi.sqlite`\n\
- `./decrypt-crvi-backup.ps1 -Mode Restore`\n\
\n\
Conseils:\n\
- Conserver les backups portables et leur passphrase separement.\n\
- Nettoyer ou migrer les anciens `*.sqlite` non chiffres.\n"
}

fn recovery_script_contents() -> &'static str {
    r#"param(
  [string]$InputPath,
  [string]$OutputPath,
  [string]$RestoreTo,
  [string]$Passphrase,
  [ValidateSet("Export", "Restore", "List")]
  [string]$Mode,
  [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

function Get-ScriptDirectory {
  if ($PSScriptRoot) {
    return $PSScriptRoot
  }
  if ($PSCommandPath) {
    return [System.IO.Path]::GetDirectoryName($PSCommandPath)
  }
  return (Get-Location).Path
}

function Convert-SecureStringToPlainText([System.Security.SecureString]$SecureValue) {
  if ($null -eq $SecureValue) {
    return ""
  }
  $ptr = [System.IntPtr]::Zero
  try {
    $ptr = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($SecureValue)
    return [System.Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
  } finally {
    if ($ptr -ne [System.IntPtr]::Zero) {
      [System.Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
    }
  }
}

function Read-HeaderFromBackup([string]$Path) {
  $magic = [System.Text.Encoding]::ASCII.GetBytes("CRVIBAK1")
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  if ($bytes.Length -lt 12) {
    throw "Fichier trop court."
  }

  for ($i = 0; $i -lt $magic.Length; $i++) {
    if ($bytes[$i] -ne $magic[$i]) {
      throw "Format non reconnu. Ce script ne traite que les backups CRVI .crvibak."
    }
  }

  $headerLength = [System.BitConverter]::ToInt32($bytes, 8)
  $headerStart = 12
  $headerEnd = $headerStart + $headerLength
  if ($bytes.Length -lt $headerEnd) {
    throw "En-tete tronque."
  }

  $headerJson = [System.Text.Encoding]::UTF8.GetString($bytes[$headerStart..($headerEnd - 1)])
  $header = $headerJson | ConvertFrom-Json
  $payloadLength = $bytes.Length - $headerEnd
  $payload = New-Object byte[] $payloadLength
  [System.Array]::Copy($bytes, $headerEnd, $payload, 0, $payloadLength)

  return @{
    Header = $header
    Payload = $payload
  }
}

function Get-BackupLabel([System.IO.FileInfo]$File) {
  if ($File.Extension -ieq ".sqlite") {
    return "SQLite non chiffre"
  }

  try {
    $headerInfo = Read-HeaderFromBackup -Path $File.FullName
    switch ($headerInfo.Header.scheme) {
      "windows-dpapi" { return "Backup CRVI protege Windows" }
      "portable-passphrase" { return "Backup CRVI portable par mot de passe" }
      default { return "Backup CRVI ($($headerInfo.Header.scheme))" }
    }
  } catch {
    return "Backup CRVI illisible"
  }
}

function Get-BackupCandidates([string]$Directory) {
  $patterns = @("*.crvibak", "*.sqlite")
  $items = foreach ($pattern in $patterns) {
    Get-ChildItem -LiteralPath $Directory -File -Filter $pattern -ErrorAction SilentlyContinue
  }

  return $items |
    Sort-Object LastWriteTime -Descending |
    Where-Object { $_.Name -notlike "restore_tmp*" }
}

function Show-BackupList([System.IO.FileInfo[]]$Candidates) {
  if (-not $Candidates -or $Candidates.Count -eq 0) {
    Write-Host "Aucun backup .crvibak ou .sqlite trouve dans ce dossier."
    return
  }

  Write-Host ""
  Write-Host "Backups detectes dans le dossier:" -ForegroundColor Cyan
  for ($i = 0; $i -lt $Candidates.Count; $i++) {
    $candidate = $Candidates[$i]
    $label = Get-BackupLabel -File $candidate
    $sizeMb = [Math]::Round($candidate.Length / 1MB, 2)
    Write-Host ("[{0}] {1} | {2} | {3} MB | {4}" -f ($i + 1), $candidate.Name, $candidate.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss"), $sizeMb, $label)
  }
  Write-Host ""
}

function Select-BackupInteractively([string]$Directory) {
  $candidates = @(Get-BackupCandidates -Directory $Directory)
  Show-BackupList -Candidates $candidates

  if (-not $candidates -or $candidates.Count -eq 0) {
    throw "Aucun backup disponible."
  }

  while ($true) {
    $answer = Read-Host "Choisir le numero du backup a restaurer"
    $index = 0
    if ([int]::TryParse($answer, [ref]$index) -and $index -ge 1 -and $index -le $candidates.Count) {
      return $candidates[$index - 1].FullName
    }
    Write-Host "Choix invalide. Reessaie avec un numero de la liste." -ForegroundColor Yellow
  }
}

function Select-ActionInteractively {
  Write-Host ""
  Write-Host "Que veux-tu faire ?" -ForegroundColor Cyan
  Write-Host "[1] Lister les backups"
  Write-Host "[2] Exporter un backup en .sqlite"
  Write-Host "[3] Restaurer un backup vers un fichier .sqlite"
  Write-Host ""

  while ($true) {
    $answer = Read-Host "Choisir 1, 2 ou 3"
    switch ($answer) {
      "1" { return "List" }
      "2" { return "Export" }
      "3" { return "Restore" }
      default {
        Write-Host "Choix invalide. Tape 1, 2 ou 3." -ForegroundColor Yellow
      }
    }
  }
}

function Get-PlainBytesFromBackup([string]$ResolvedInputPath, [string]$ProvidedPassphrase) {
  $extension = [System.IO.Path]::GetExtension($ResolvedInputPath)
  if ($extension -ieq ".sqlite") {
    return [System.IO.File]::ReadAllBytes($ResolvedInputPath)
  }

  $headerInfo = Read-HeaderFromBackup -Path $ResolvedInputPath
  $header = $headerInfo.Header
  $payload = $headerInfo.Payload

  switch ($header.scheme) {
    "windows-dpapi" {
      Add-Type -AssemblyName System.Security
      return [System.Security.Cryptography.ProtectedData]::Unprotect(
        $payload,
        $null,
        [System.Security.Cryptography.DataProtectionScope]::CurrentUser
      )
    }
    "portable-passphrase" {
      $effectivePassphrase = $ProvidedPassphrase
      if ([string]::IsNullOrWhiteSpace($effectivePassphrase)) {
        $securePassphrase = Read-Host "Mot de passe du backup portable" -AsSecureString
        $effectivePassphrase = Convert-SecureStringToPlainText -SecureValue $securePassphrase
      }
      if ([string]::IsNullOrWhiteSpace($effectivePassphrase)) {
        throw "Ce backup portable requiert un mot de passe."
      }

      $salt = [Convert]::FromBase64String($header.salt_b64)
      $nonce = [Convert]::FromBase64String($header.nonce_b64)
      $iterations = [int]$header.pbkdf2_iterations
      $derive = [System.Security.Cryptography.Rfc2898DeriveBytes]::new(
        $effectivePassphrase,
        $salt,
        $iterations,
        [System.Security.Cryptography.HashAlgorithmName]::SHA256
      )
      try {
        $key = $derive.GetBytes(32)
      } finally {
        $derive.Dispose()
      }

      $tag = New-Object byte[] 16
      $cipherLength = $payload.Length - 16
      if ($cipherLength -le 0) {
        throw "Payload AES-GCM invalide."
      }
      $cipher = New-Object byte[] $cipherLength
      $plain = New-Object byte[] $cipherLength
      [System.Array]::Copy($payload, 0, $cipher, 0, $cipherLength)
      [System.Array]::Copy($payload, $cipherLength, $tag, 0, 16)

      $aes = [System.Security.Cryptography.AesGcm]::new($key)
      try {
        $aes.Decrypt($nonce, $cipher, $tag, $plain)
        return $plain
      } finally {
        $aes.Dispose()
      }
    }
    default {
      throw "Schema inconnu: $($header.scheme)"
    }
  }
}

function Get-DefaultOutputPath([string]$ResolvedInputPath) {
  $directory = [System.IO.Path]::GetDirectoryName($ResolvedInputPath)
  $baseName = [System.IO.Path]::GetFileNameWithoutExtension($ResolvedInputPath)
  return [System.IO.Path]::Combine($directory, "$baseName.sqlite")
}

function Prompt-RestoreDestination([string]$ResolvedInputPath) {
  $suggestedPath = Get-DefaultOutputPath -ResolvedInputPath $ResolvedInputPath
  $answer = Read-Host "Chemin du fichier .sqlite a restaurer (Entrée = $suggestedPath)"
  if ([string]::IsNullOrWhiteSpace($answer)) {
    return $suggestedPath
  }
  return $answer
}

$scriptDirectory = Get-ScriptDirectory

if ($ListOnly -and [string]::IsNullOrWhiteSpace($Mode)) {
  $Mode = "List"
}

if ([string]::IsNullOrWhiteSpace($Mode) -and [string]::IsNullOrWhiteSpace($InputPath)) {
  $Mode = Select-ActionInteractively
}

if ($Mode -eq "List") {
  $candidates = @(Get-BackupCandidates -Directory $scriptDirectory)
  Show-BackupList -Candidates $candidates
  exit 0
}

if ([string]::IsNullOrWhiteSpace($InputPath)) {
  $InputPath = Select-BackupInteractively -Directory $scriptDirectory
}

$resolvedInput = (Resolve-Path -LiteralPath $InputPath).Path
$plain = Get-PlainBytesFromBackup -ResolvedInputPath $resolvedInput -ProvidedPassphrase $Passphrase

if ($Mode -eq "Restore" -and [string]::IsNullOrWhiteSpace($RestoreTo)) {
  $RestoreTo = Prompt-RestoreDestination -ResolvedInputPath $resolvedInput
}

if (-not [string]::IsNullOrWhiteSpace($RestoreTo)) {
  $destinationDirectory = [System.IO.Path]::GetDirectoryName($RestoreTo)
  if (-not [string]::IsNullOrWhiteSpace($destinationDirectory)) {
    [System.IO.Directory]::CreateDirectory($destinationDirectory) | Out-Null
  }
  [System.IO.File]::WriteAllBytes($RestoreTo, $plain)
  Write-Host "Base restauree vers: $RestoreTo"
  exit 0
}

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Get-DefaultOutputPath -ResolvedInputPath $resolvedInput
}

[System.IO.File]::WriteAllBytes($OutputPath, $plain)
Write-Host "Backup exporte vers: $OutputPath"
"#
}
