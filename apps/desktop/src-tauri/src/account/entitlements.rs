use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Claims of the signed offline entitlement document issued by the server
/// (JWS compact, EdDSA, `typ: forma-entitlement+jwt`). The private key never
/// leaves the server; the desktop only holds public keys (JWKS).
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub ver: u32,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
    pub rfa: i64,
    pub plan: String,
    pub subscription_status: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Capability {
    pub key: String,
    pub limit: Option<i64>,
    pub until: Option<String>,
}

#[derive(Deserialize)]
struct Header {
    alg: String,
    kid: Option<String>,
    typ: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum DocState {
    /// Signature, issuer, audience and time window are valid.
    Valid,
    /// Valid, but the client should refresh online (`rfa` passed).
    Stale,
    /// Past `exp`: paid local features are off until the next online check.
    Expired,
    /// The clock went backwards beyond tolerance; treated as not valid.
    ClockRollback,
    /// Bad signature, unknown key, wrong issuer/audience or unsupported version.
    Invalid,
}

const SKEW: i64 = 300;
const SUPPORTED_VERSION: u32 = 1;

fn find_key(jwks: &[serde_json::Value], kid: &str) -> Option<VerifyingKey> {
    let jwk = jwks
        .iter()
        .find(|k| k.get("kid").and_then(|v| v.as_str()) == Some(kid))?;
    if jwk.get("kty")?.as_str()? != "OKP" || jwk.get("crv")?.as_str()? != "Ed25519" {
        return None;
    }
    let x = URL_SAFE_NO_PAD.decode(jwk.get("x")?.as_str()?).ok()?;
    VerifyingKey::from_bytes(&x.try_into().ok()?).ok()
}

/// Verifies the document and evaluates it at `now` (unix seconds).
/// `max_seen` is the latest time this installation has observed; a clock set
/// back further than the tolerance is reported instead of trusted.
pub fn evaluate(
    document: &str,
    jwks: &[serde_json::Value],
    issuer: &str,
    now: i64,
    max_seen: i64,
) -> (DocState, Option<Claims>) {
    let parts: Vec<&str> = document.split('.').collect();
    if parts.len() != 3 {
        return (DocState::Invalid, None);
    }
    let Some(header) = URL_SAFE_NO_PAD
        .decode(parts[0])
        .ok()
        .and_then(|b| serde_json::from_slice::<Header>(&b).ok())
    else {
        return (DocState::Invalid, None);
    };
    if header.alg != "EdDSA" || header.typ.as_deref() != Some("forma-entitlement+jwt") {
        return (DocState::Invalid, None);
    }
    let Some(key) = header.kid.as_deref().and_then(|kid| find_key(jwks, kid)) else {
        return (DocState::Invalid, None);
    };
    let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(parts[2]) else {
        return (DocState::Invalid, None);
    };
    let Ok(signature) = Signature::from_slice(&sig_bytes) else {
        return (DocState::Invalid, None);
    };
    let signed = format!("{}.{}", parts[0], parts[1]);
    if key.verify(signed.as_bytes(), &signature).is_err() {
        return (DocState::Invalid, None);
    }
    let Some(claims) = URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()
        .and_then(|b| serde_json::from_slice::<Claims>(&b).ok())
    else {
        return (DocState::Invalid, None);
    };
    if claims.iss != issuer || claims.aud != "forma-desktop" || claims.ver != SUPPORTED_VERSION {
        return (DocState::Invalid, Some(claims));
    }
    let state = if now + SKEW < max_seen {
        DocState::ClockRollback
    } else if now >= claims.exp || now + SKEW < claims.nbf {
        DocState::Expired
    } else if now >= claims.rfa {
        DocState::Stale
    } else {
        DocState::Valid
    };
    (state, Some(claims))
}

/// Capabilities usable by local features right now (empty unless the document is valid or stale).
pub fn usable_capabilities(state: DocState, claims: Option<&Claims>) -> Vec<String> {
    match (state, claims) {
        (DocState::Valid | DocState::Stale, Some(c)) => {
            c.capabilities.iter().map(|c| c.key.clone()).collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sign(key: &SigningKey, kid: &str, typ: &str, claims: &serde_json::Value) -> String {
        let h = URL_SAFE_NO_PAD
            .encode(serde_json::json!({ "alg": "EdDSA", "kid": kid, "typ": typ }).to_string());
        let p = URL_SAFE_NO_PAD.encode(claims.to_string());
        let sig = key.sign(format!("{h}.{p}").as_bytes());
        format!("{h}.{p}.{}", URL_SAFE_NO_PAD.encode(sig.to_bytes()))
    }

    fn jwk(key: &SigningKey, kid: &str) -> serde_json::Value {
        serde_json::json!({ "kty": "OKP", "crv": "Ed25519", "kid": kid, "x": URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()) })
    }

    fn claims(now: i64) -> serde_json::Value {
        serde_json::json!({
            "iss": "https://cad.kartochk1n.ru", "aud": "forma-desktop", "sub": "u1", "jti": "j", "ver": 1,
            "iat": now, "nbf": now - 60, "exp": now + 7 * 86400, "rfa": now + 86400,
            "plan": "pro", "subscriptionStatus": "active",
            "capabilities": [{ "key": "support.priority", "limit": null, "until": null }]
        })
    }

    #[test]
    fn evaluates_valid_stale_expired_tampered_foreign_and_rotated() {
        let now = 1_800_000_000;
        let k1 = SigningKey::from_bytes(&[7u8; 32]);
        let k2 = SigningKey::from_bytes(&[9u8; 32]);
        let iss = "https://cad.kartochk1n.ru";
        let doc = sign(&k1, "ent-1", "forma-entitlement+jwt", &claims(now));
        let jwks = vec![jwk(&k1, "ent-1")];

        let (s, c) = evaluate(&doc, &jwks, iss, now, now);
        assert_eq!(s, DocState::Valid);
        assert_eq!(usable_capabilities(s, c.as_ref()), vec!["support.priority"]);
        assert_eq!(
            evaluate(&doc, &jwks, iss, now + 2 * 86400, now).0,
            DocState::Stale
        );
        assert_eq!(
            evaluate(&doc, &jwks, iss, now + 8 * 86400, now).0,
            DocState::Expired
        );
        assert!(usable_capabilities(DocState::Expired, c.as_ref()).is_empty());
        // Clock moved back a day after we had seen a later time.
        assert_eq!(
            evaluate(&doc, &jwks, iss, now - 86400, now).0,
            DocState::ClockRollback
        );

        // Tampered payload (client-side "isPro") breaks the signature.
        let mut forged = claims(now);
        forged["capabilities"] =
            serde_json::json!([{ "key": "cloud.ai_credits", "limit": null, "until": null }]);
        let parts: Vec<&str> = doc.split('.').collect();
        let tampered = format!(
            "{}.{}.{}",
            parts[0],
            URL_SAFE_NO_PAD.encode(forged.to_string()),
            parts[2]
        );
        assert_eq!(
            evaluate(&tampered, &jwks, iss, now, now).0,
            DocState::Invalid
        );

        // Signed by a foreign key with the same kid, wrong typ, wrong issuer.
        assert_eq!(
            evaluate(
                &sign(&k2, "ent-1", "forma-entitlement+jwt", &claims(now)),
                &jwks,
                iss,
                now,
                now
            )
            .0,
            DocState::Invalid
        );
        assert_eq!(
            evaluate(
                &sign(&k1, "ent-1", "at+jwt", &claims(now)),
                &jwks,
                iss,
                now,
                now
            )
            .0,
            DocState::Invalid
        );
        assert_eq!(
            evaluate(&doc, &jwks, "https://evil.example", now, now).0,
            DocState::Invalid
        );

        // Rotation: new key signs, both keys published.
        let rotated = vec![jwk(&k2, "ent-2"), jwk(&k1, "ent-1")];
        assert_eq!(evaluate(&doc, &rotated, iss, now, now).0, DocState::Valid);
        assert_eq!(
            evaluate(
                &sign(&k2, "ent-2", "forma-entitlement+jwt", &claims(now)),
                &rotated,
                iss,
                now,
                now
            )
            .0,
            DocState::Valid
        );
        assert_eq!(
            evaluate(
                &sign(&k2, "ent-2", "forma-entitlement+jwt", &claims(now)),
                &jwks,
                iss,
                now,
                now
            )
            .0,
            DocState::Invalid
        );
    }
}
