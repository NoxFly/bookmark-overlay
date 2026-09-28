// Bookmark Overlay
// Copyright (C) 2026 NoxFly
//
// FR : Ce programme est un logiciel libre ; vous pouvez le redistribuer ou le
// modifier selon les termes de la GNU Affero General Public License, version 3,
// telle que publiée par la Free Software Foundation. Il est distribué dans
// l'espoir d'être utile, mais SANS AUCUNE GARANTIE. Voir le fichier LICENSE.
//
// EN : This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License, version 3, as
// published by the Free Software Foundation. It is distributed in the hope that
// it will be useful, but WITHOUT ANY WARRANTY. See the LICENSE file.
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Chiffrement du fichier de données.
//!
//! Le référentiel clients ne doit pas être lisible en ouvrant un fichier, ni
//! décodable par un utilitaire en ligne comme le serait du base64. Il est donc
//! chiffré en AES-256-GCM, un chiffrement *authentifié* : une altération du
//! fichier est détectée au déchiffrement au lieu de produire des données fausses.
//!
//! # Ce que cela protège, et ce que cela ne protège pas
//!
//! La clé est injectée à la compilation (voir `build.rs`) : absente du dépôt, qui
//! est public, elle est présente dans le binaire. Elle arrête quiconque tombe sur
//! le fichier — une session laissée ouverte, une sauvegarde recopiée, un disque
//! revendu — mais pas quelqu'un qui possède l'exécutable et cherche vraiment : il
//! peut l'en extraire. C'est un choix assumé, la contrepartie étant qu'aucun mot
//! de passe n'est demandé au démarrage et qu'un export reste lisible par
//! l'application chez un collègue, dont le binaire embarque la même clé.
//!
//! Deux formats coexistent : `V2`, chiffré avec la clé courante, et `V1`, écrit
//! par Customers Overlay avec une clé qui était dans le code source. Un fichier
//! `V1` n'est lisible que si le build connaît cette ancienne clé ; il est alors
//! réécrit en `V2` à la première lecture.
//!
//! Pour aller plus loin il faudrait lier la clé à la session Windows (DPAPI), au
//! prix d'un fichier non transmissible, ou la dériver d'un mot de passe saisi à
//! chaque démarrage.

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};

use crate::error::{validation, AppResult};

/// Marqueur du format courant, chiffré avec [`KEY`].
const MAGIC: &[u8; 8] = b"BOOKOV2\x00";

/// Marqueur du format de Customers Overlay, chiffré avec [`LEGACY_KEY`].
const LEGACY_MAGIC: &[u8; 8] = b"CUSTOV1\x00";

/// Taille du nonce d'AES-GCM.
const NONCE_LEN: usize = 12;

// `KEY` et `LEGACY_KEY`, générées par `build.rs` depuis l'environnement ou
// `.env.local`.
include!(concat!(env!("OUT_DIR"), "/keys.rs"));

/// Vrai si ces octets sont un fichier chiffré par l'application.
pub fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC) || is_legacy(bytes)
}

/// Vrai si ces octets sont au format de Customers Overlay, à réécrire.
pub fn is_legacy(bytes: &[u8]) -> bool {
    bytes.starts_with(LEGACY_MAGIC)
}

/// Chiffre `plaintext`, en préfixant le résultat du marqueur et du nonce.
///
/// Un nonce aléatoire est tiré à chaque écriture : deux enregistrements du même
/// contenu ne produisent pas le même fichier, et le nonce n'est jamais réutilisé.
pub fn seal(plaintext: &[u8]) -> AppResult<Vec<u8>> {
    seal_with(MAGIC, &KEY, plaintext)
}

fn seal_with(magic: &[u8; 8], key: &[u8; 32], plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| validation("Le chiffrement des données a échoué."))?;

    let mut sealed = Vec::with_capacity(magic.len() + NONCE_LEN + ciphertext.len());
    sealed.extend_from_slice(magic);
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);
    Ok(sealed)
}

/// Déchiffre un fichier produit par [`seal`].
///
/// # Erreurs
///
/// Échoue si le marqueur est absent, si le fichier est tronqué, ou si le contenu
/// a été altéré — l'authentification d'AES-GCM le détecte.
pub fn open(sealed: &[u8]) -> AppResult<Vec<u8>> {
    open_with(sealed, &KEY, LEGACY_KEY.as_ref())
}

fn open_with(sealed: &[u8], key: &[u8; 32], legacy_key: Option<&[u8; 32]>) -> AppResult<Vec<u8>> {
    let key = if sealed.starts_with(MAGIC) {
        key
    } else if is_legacy(sealed) {
        legacy_key.ok_or_else(|| {
            validation(
                "Ce fichier vient de Customers Overlay et cette version ne connaît pas son ancienne clé.",
            )
        })?
    } else {
        return Err(validation(
            "Ce fichier n'est pas un fichier chiffré de l'application.",
        ));
    };
    let body = &sealed[MAGIC.len()..];
    if body.len() <= NONCE_LEN {
        return Err(validation("Fichier chiffré tronqué."));
    }
    let (nonce, ciphertext) = body.split_at(NONCE_LEN);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| validation("Fichier chiffré illisible : contenu altéré ou clé différente."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_then_open_restores_the_plaintext() {
        let plaintext = b"{\"customers\":[]}";

        let sealed = seal(plaintext).expect("chiffrement");
        let opened = open(&sealed).expect("dechiffrement");

        assert_eq!(opened, plaintext);
    }

    #[test]
    fn sealed_bytes_do_not_contain_the_plaintext() {
        let plaintext = b"Acme Industries";
        let sealed = seal(plaintext).expect("chiffrement");
        assert!(!sealed
            .windows(plaintext.len())
            .any(|window| window == plaintext));
    }

    #[test]
    fn two_seals_of_the_same_content_differ() {
        let plaintext = b"identique";
        assert_ne!(
            seal(plaintext).expect("chiffrement 1"),
            seal(plaintext).expect("chiffrement 2")
        );
    }

    #[test]
    fn open_rejects_plain_json() {
        assert!(open(b"{\"customers\":[]}").is_err());
    }

    #[test]
    fn open_rejects_a_truncated_file() {
        let mut sealed = seal(b"peu importe").expect("chiffrement");
        sealed.truncate(MAGIC.len() + 4);
        assert!(open(&sealed).is_err());
    }

    #[test]
    fn open_detects_tampering() {
        let mut sealed = seal(b"contenu authentique").expect("chiffrement");
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert!(open(&sealed).is_err());
    }

    const OLD: [u8; 32] = [7; 32];
    const NEW: [u8; 32] = [9; 32];

    #[test]
    fn a_legacy_file_opens_with_the_legacy_key() {
        let sealed = seal_with(LEGACY_MAGIC, &OLD, b"ancien").expect("chiffrement");
        assert!(is_legacy(&sealed));
        assert_eq!(
            open_with(&sealed, &NEW, Some(&OLD)).expect("lecture"),
            b"ancien"
        );
    }

    #[test]
    fn a_legacy_file_is_refused_without_the_legacy_key() {
        let sealed = seal_with(LEGACY_MAGIC, &OLD, b"ancien").expect("chiffrement");
        assert!(open_with(&sealed, &NEW, None).is_err());
    }

    #[test]
    fn a_current_file_never_uses_the_legacy_key() {
        let sealed = seal_with(MAGIC, &NEW, b"courant").expect("chiffrement");
        assert!(!is_legacy(&sealed));
        assert!(open_with(&sealed, &OLD, Some(&NEW)).is_err());
    }

    #[test]
    fn is_encrypted_tells_the_two_formats_apart() {
        assert!(is_encrypted(&seal(b"x").expect("chiffrement")));
        assert!(!is_encrypted(b"{\"customers\":[]}"));
    }
}
