//! «Gi oss tilbakemelding»: kunden skriver selv, ser nøyaktig hva som sendes, og meldingen
//! åpnes som e-post i kundens eget e-postprogram. Appen sender ingenting selv, og ingen bilder,
//! filstier eller navn fra familieprofilen er med (docs/ADMIN.md).

use crate::commands::{CmdResult, CommandError};

/// Adressen tilbakemeldinger går til (eierens valg 12. oktober 2026).
pub const FEEDBACK_EMAIL: &str = "plussdelta@fo2album.no";

/// Prosentkoding for `mailto:` (RFC 6068): bare bokstaver, tall og `-._~` står som de er.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `mailto:`-lenken med emne og tekst.
pub fn mailto(subject: &str, body: &str) -> String {
    // Linjeskift i e-post skal være CRLF.
    let body = body.replace("\r\n", "\n").replace('\n', "\r\n");
    format!(
        "mailto:{FEEDBACK_EMAIL}?subject={}&body={}",
        encode(subject),
        encode(&body)
    )
}

/// Åpner meldingen i kundens e-postprogram.
#[tauri::command]
pub fn open_feedback_mail(subject: String, body: String) -> CmdResult<()> {
    let url = mailto(&subject, &body);
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(&url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler").arg(&url);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(&url);
        c
    };
    cmd.spawn()
        .map_err(|e| CommandError::new("ingen_epost", e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailto_koder_norske_tegn_og_linjeskift() {
        let url = mailto("Forslag til Fo2Album", "Hei!\nBlå & grønn?");
        assert_eq!(
            url,
            "mailto:plussdelta@fo2album.no?subject=Forslag%20til%20Fo2Album\
             &body=Hei%21%0D%0ABl%C3%A5%20%26%20gr%C3%B8nn%3F"
        );
    }
}
