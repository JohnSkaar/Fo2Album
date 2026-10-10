# Testversjon av Fo2Album

Testversjonen er appen slik den er nå, bygget for Mac og Windows av GitHub (jobben
«Testversjon av appen» i `.github/workflows/ci.yml`). Den startes manuelt: GitHub › Actions ›
CI › «Run workflow» på grenen. Filene ligger under «Artifacts» nederst på kjøringen:
`Fo2Album-Mac` (`.dmg`) og `Fo2Album-Windows` (`.exe`-installasjon).

Alt skjer på maskinen. Bildene leses der de ligger og kopieres ikke. Det eneste appen kan lage
for deling, er trykkfilen (PDF), og den lagres bare der du velger.

## Installere

Appen er ikke signert ennå (det krever et sertifikat fra Apple og Microsoft), så maskinen
advarer første gang.

**Mac**
1. Last ned `Fo2Album-Mac`, pakk ut zip-filen og åpne `.dmg`-filen.
2. Dra Fo2Album til Programmer.
3. Åpne appen. Sier macOS at den ikke kan åpnes: gå til Systeminnstillinger › Personvern og
   sikkerhet, bla ned og trykk «Åpne likevel». (Sier macOS at appen «er skadet», kjør
   `xattr -cr /Applications/Fo2Album.app` i Terminal og prøv igjen.)

**Windows**
1. Last ned `Fo2Album-Windows`, pakk ut zip-filen og kjør installasjonsfilen.
2. Sier Windows «Windows beskyttet PC-en», trykk «Mer informasjon» og «Kjør likevel».

## Hva du kan prøve

1. Lag lagringen og skriv ned gjenopprettingsnøkkelen.
2. Velg bildemapper: mappe på maskinen, Dropbox, iCloud Bilder eller Google Disk. Bilder som bare
   ligger i skyen, må lastes ned til maskinen først (Dropbox/OneDrive: «Gjør tilgjengelig
   frakoblet»).
3. Vent til bildene er hentet. Ansiktene finnes underveis (gruppebilder tar litt lenger tid).
4. «Hvem er med?»: gi navn og rolle til de viktigste personene (særlig «Barn i familien»).
5. Velg år og trykk «Lag utkast». Se på historiene, sidene og begrunnelsene. Bytt, ta med, ta
   bort, dra bilder mellom sider, prøv «Rammer», «Større», «Fornøyd» og «Angre».
6. «Før trykk»: se hva kvalitetssjekken sier.
7. «Bla i albumet»: skriv teksten på forsiden, første side og baksiden.
8. «Lag trykkfil» og se på PDF-en.

Meld gjerne fra om: historier som er delt feil, bilder som burde vært med (eller ikke), personer
som er slått sammen eller delt i flere, sider som ser rare ut, og hvor lang tid innlesingen tok
(antall bilder og maskin).

## Data og sletting

Lagringen er kryptert og ligger i appens datamappe (Mac: `~/Library/Application Support/
no.fo2album.app`, Windows: `%APPDATA%\no.fo2album.app`). «Slett alle data» nederst i menyen
fjerner alt appen har laget. Bildene dine røres ikke.
