# Evaluering mot familiens egne album (M2)

Målet er å måle hvor godt utvalget treffer det familien selv valgte (SCORING.md §10).
Alt skjer **på din egen maskin**: albumene, bildene og gullsettene forlater den aldri.
Bare tallene (prosenter og antall) skrives til `eval/RESULTS.md`.

## Slik lager du et gullsett

1. **Les inn bildene fra året** (og gjerne året før og etter, i tilfelle kameraklokken var feil),
   enten i appen eller med verktøyet:
   ```
   p2a les-inn --mappe "D:\Bilder\2010"
   ```
2. **Last ned albumet som PDF** (Blurb: «Download PDF») og lag gullsettet:
   ```
   p2a eval lag-gullsett --pdf "Øyeblikk fra 2010.pdf" --aar 2010 --navn familie-2010
   ```
   Verktøyet henter bildene ut av PDF-en, finner hvilke bilder i biblioteket de er laget av
   (også når albumet har beskåret dem), og lagrer resultatet kryptert i appens database.
   Det skriver også en **kontrollside** (HTML) med usikre treff og albumbilder uten treff.
   Åpne den og se om noe ser feil ut. Kontrollsiden inneholder bilder og ligger bare i
   appens datamappe.
3. **Gjenta** for hvert år (2006–2010).

## Slik kjører du evalueringen

```
p2a eval kjor --resultater eval/RESULTS.md
```

Kjører utvalget mot alle gullsettene med like mange bilder som familien valgte, og legger
en tabell til i `eval/RESULTS.md`. Commit filen, så vi ser utviklingen over tid.

## Hva tallene betyr

| Mål | Betydning |
|---|---|
| Presisjon | Andel av utvalget som familien også valgte. «Nesten samme bilde» (samme serie innen 20 s, eller nesten lik) teller som treff |
| Gjenfinning | Andel av familiens valg som er med i utvalget |
| Hendelser med (utvalg) | Andel av årets hendelser (minst 3 bilder) som utvalget har med. **Viktigst** etter eierens føring: hver hendelse skal ha minst én side |
| Hendelser med (familien) | Samme for familiens eget album. Sjekker at «nesten alle hendelser er med» stemmer |
| Familiens hendelser gjenfunnet | Andel av hendelsene i familiens album som utvalget også har med |
| Måneder gjenfunnet | Andel av månedene i familiens album som utvalget har med |
| Redundans | Hvor like de valgte bildene er hverandre (0–1, lavere er bedre) |

Variasjon i bildetyper (M3) og sjeldne personer (M4) kommer når de finnes.

## Uten Rust på maskinen

Verktøyet `p2a` bygges for Mac og Windows i GitHub Actions: åpne *Actions › CI › Run workflow*
på grenen, vent til kjøringen er ferdig, og last ned `p2a-macos` eller `p2a-windows` under
*Artifacts*. Mac: gi filen kjøretillatelse med `chmod +x p2a`.
