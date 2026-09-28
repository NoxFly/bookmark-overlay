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

//! Persistance du référentiel clients dans un unique fichier JSON.
//!
//! Le fichier est chargé une seule fois au démarrage puis maintenu en mémoire :
//! la recherche dans l'overlay ne touche jamais le disque. Chaque écriture est
//! atomique (fichier temporaire puis renommage) pour qu'une coupure ne laisse
//! jamais un `data.json` tronqué.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::crypto;
use crate::error::{validation, AppError, AppResult};
use crate::model::{Customer, Database, Settings};

/// Taille maximale acceptée pour un fichier de données lu ou importé (16 Mio).
/// Au-delà, on refuse plutôt que de charger un fichier arbitraire en mémoire.
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Nombre maximal de fiches conservées.
const MAX_CUSTOMERS: usize = 10_000;

/// Nom de l'ancien fichier en clair, repris au premier démarrage chiffré.
const LEGACY_FILE: &str = "data.json";

/// Copie de l'ancien fichier, conservée après la migration.
const LEGACY_BACKUP: &str = "data.json.avant-chiffrement";

/// Dépôt de données thread-safe, partagé comme état Tauri.
pub struct Store {
    path: PathBuf,
    database: RwLock<Database>,
    fresh: bool,
}

impl Store {
    /// Charge le fichier de données, ou en crée un vide s'il n'existe pas encore.
    ///
    /// Le fichier est écrit dès ce premier chargement : le dossier de données est
    /// alors consultable tout de suite, et `is_fresh` ne répond « oui » qu'une
    /// seule fois dans la vie de l'installation.
    ///
    /// Un fichier présent mais illisible n'est jamais écrasé silencieusement :
    /// l'erreur remonte pour que l'utilisateur puisse récupérer son contenu.
    pub fn load(path: PathBuf) -> AppResult<Self> {
        let adopted = adopt_legacy_file(&path)?;

        let mut fresh = false;
        let mut migrated = adopted;
        let database = match read_database(&path) {
            Ok(loaded) => {
                migrated = loaded.migrated;
                loaded.database
            }
            Err(AppError::Io(error)) if error.kind() == ErrorKind::NotFound => {
                fresh = true;
                Database::default()
            }
            Err(error) => return Err(error),
        };
        // Les clés attribuées à la lecture doivent être écrites tout de suite,
        // faute de quoi elles changeraient à chaque démarrage.
        if fresh || migrated {
            write_atomically(&path, &database)?;
        }
        Ok(Self {
            path,
            database: RwLock::new(database),
            fresh,
        })
    }

    /// Chemin du fichier de données.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Vrai si aucun fichier de données n'existait au chargement, autrement dit
    /// s'il s'agit du tout premier lancement sur cette machine.
    pub fn is_fresh(&self) -> bool {
        self.fresh
    }

    /// Copie de l'intégralité de la base (clients + réglages).
    pub fn snapshot(&self) -> AppResult<Database> {
        let guard = self.database.read().map_err(|_| AppError::PoisonedState)?;
        Ok(guard.clone())
    }

    /// Réglages courants.
    pub fn settings(&self) -> AppResult<Settings> {
        let guard = self.database.read().map_err(|_| AppError::PoisonedState)?;
        Ok(guard.settings.clone())
    }

    /// Retourne la fiche portant cette clé.
    pub fn find(&self, id: &str) -> AppResult<Customer> {
        let guard = self.database.read().map_err(|_| AppError::PoisonedState)?;
        guard
            .customers
            .iter()
            .find(|customer| customer.has_id(id))
            .cloned()
            .ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    /// Ajoute une fiche. Échoue si le tenant est déjà pris.
    pub fn create(&self, customer: Customer) -> AppResult<()> {
        let customer = customer.sanitized()?;
        self.mutate(|database| {
            if database.customers.len() >= MAX_CUSTOMERS {
                return Err(validation(format!(
                    "Limite de {MAX_CUSTOMERS} clients atteinte."
                )));
            }
            if database
                .customers
                .iter()
                .any(|existing| existing.has_tenant(&customer.tenant))
            {
                return Err(AppError::DuplicateTenant(customer.tenant.clone()));
            }
            database.customers.push(customer);
            Ok(())
        })
    }

    /// Remplace la fiche identifiée par `id`. Le tenant peut changer, à condition
    /// que le nouveau ne soit pas déjà utilisé par une autre fiche.
    pub fn update(&self, id: &str, customer: Customer) -> AppResult<()> {
        // La clé vient du dépôt, jamais de la charge utile : une fiche ne change
        // pas d'identité parce que le front a omis ou modifié son identifiant.
        let customer = Customer {
            id: id.trim().to_owned(),
            ..customer
        }
        .sanitized()?;
        self.mutate(|database| {
            let index = database
                .customers
                .iter()
                .position(|existing| existing.has_id(id))
                .ok_or_else(|| AppError::NotFound(id.to_owned()))?;

            let conflict = database
                .customers
                .iter()
                .enumerate()
                .any(|(position, existing)| {
                    position != index && existing.has_tenant(&customer.tenant)
                });
            if conflict {
                return Err(AppError::DuplicateTenant(customer.tenant.clone()));
            }

            database.customers[index] = customer;
            Ok(())
        })
    }

    /// Supprime la fiche identifiée par `id`.
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.mutate(|database| {
            let index = database
                .customers
                .iter()
                .position(|existing| existing.has_id(id))
                .ok_or_else(|| AppError::NotFound(id.to_owned()))?;
            database.customers.remove(index);
            Ok(())
        })
    }

    /// Enregistre de nouveaux réglages et retourne la version normalisée.
    pub fn save_settings(&self, settings: Settings) -> AppResult<Settings> {
        let settings = settings.sanitized()?;
        let stored = settings.clone();
        self.mutate(move |database| {
            database.settings = settings;
            Ok(())
        })?;
        Ok(stored)
    }

    /// Importe un jeu de données.
    ///
    /// En mode fusion, les fiches existantes sont conservées et seules les
    /// nouvelles clés sont ajoutées ; en mode remplacement, tout est écrasé.
    /// Dans les deux cas, une sauvegarde horodatée du fichier courant est écrite
    /// avant toute modification.
    pub fn import(&self, incoming: Database, mode: ImportMode) -> AppResult<ImportReport> {
        let incoming = incoming.sanitized()?;
        let backup = self.write_backup()?;

        let mut report = ImportReport {
            added: 0,
            updated: 0,
            skipped: 0,
            backup_path: backup.to_string_lossy().into_owned(),
        };

        self.mutate(|database| {
            match mode {
                ImportMode::Replace => {
                    report.added = incoming.customers.len();
                    database.customers = incoming.customers;
                    database.settings = incoming.settings;
                }
                ImportMode::Merge => {
                    for customer in incoming.customers {
                        // Appariement par clé, avec repli sur le tenant : un fichier
                        // exporté avant l'introduction des clés n'en porte pas.
                        match database.customers.iter().position(|existing| {
                            existing.has_id(&customer.id) || existing.has_tenant(&customer.tenant)
                        }) {
                            Some(index) => {
                                if database.customers[index] == customer {
                                    report.skipped += 1;
                                } else {
                                    database.customers[index] = customer;
                                    report.updated += 1;
                                }
                            }
                            None => {
                                database.customers.push(customer);
                                report.added += 1;
                            }
                        }
                    }
                    if database.customers.len() > MAX_CUSTOMERS {
                        return Err(validation(format!(
                            "L'import dépasserait la limite de {MAX_CUSTOMERS} clients."
                        )));
                    }
                }
            }
            Ok(())
        })?;

        Ok(report)
    }

    /// Écrit une copie horodatée du fichier de données à côté de celui-ci.
    /// Si aucun fichier n'existe encore, la sauvegarde contient l'état en mémoire.
    pub fn write_backup(&self) -> AppResult<PathBuf> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        let target = self
            .path
            .with_file_name(format!("data.backup-{stamp}.json"));

        match fs::copy(&self.path, &target) {
            Ok(_) => Ok(target),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let snapshot = self.snapshot()?;
                write_atomically(&target, &snapshot)?;
                Ok(target)
            }
            Err(error) => Err(AppError::Io(error)),
        }
    }

    /// Applique une mutation à la base puis l'écrit sur disque.
    ///
    /// L'état en mémoire n'est publié que si l'écriture disque a réussi : en cas
    /// d'échec, le verrou est relâché sur les données d'origine.
    fn mutate<F>(&self, apply: F) -> AppResult<()>
    where
        F: FnOnce(&mut Database) -> AppResult<()>,
    {
        let mut guard = self.database.write().map_err(|_| AppError::PoisonedState)?;
        let mut candidate = guard.clone();
        apply(&mut candidate)?;
        sort_customers(&mut candidate.customers);
        write_atomically(&self.path, &candidate)?;
        *guard = candidate;
        Ok(())
    }
}

/// Stratégie appliquée lors d'un import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Conserve les fiches existantes et complète avec le fichier importé.
    Merge,
    /// Remplace intégralement la base par le fichier importé.
    Replace,
}

/// Bilan d'un import, affiché à l'utilisateur.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Fiches créées.
    pub added: usize,
    /// Fiches existantes mises à jour.
    pub updated: usize,
    /// Fiches identiques, laissées telles quelles.
    pub skipped: usize,
    /// Chemin de la sauvegarde écrite avant l'import.
    pub backup_path: String,
}

/// Trie les fiches par nom puis par tenant, pour un fichier JSON stable et diffable.
fn sort_customers(customers: &mut [Customer]) {
    customers.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.tenant.to_lowercase().cmp(&right.tenant.to_lowercase()))
    });
}

/// Reprend l'ancien fichier en clair s'il est le seul présent.
///
/// Il est d'abord copié à côté, sous un nom explicite, puis relu pour devenir le
/// fichier chiffré. L'original est ensuite supprimé : le laisser en place viderait
/// le chiffrement de son sens. La copie, elle, reste lisible — elle est là pour
/// pouvoir revenir en arrière, et c'est à l'utilisateur de la supprimer.
fn adopt_legacy_file(path: &Path) -> AppResult<bool> {
    if path.exists() {
        return Ok(false);
    }
    let Some(directory) = path.parent() else {
        return Ok(false);
    };
    let legacy = directory.join(LEGACY_FILE);
    if !legacy.is_file() {
        return Ok(false);
    }

    let contents = fs::read(&legacy)?;
    let database: Database = serde_json::from_str(&decode(&contents)?)?;
    let database = database.sanitized()?;

    fs::copy(&legacy, directory.join(LEGACY_BACKUP))?;
    write_atomically(path, &database)?;
    fs::remove_file(&legacy)?;
    Ok(true)
}

/// Résultat de la lecture d'un fichier de données.
pub struct LoadedDatabase {
    /// Le contenu validé.
    pub database: Database,
    /// Vrai si le fichier doit être réécrit : clés techniques attribuées à la
    /// lecture, ou chiffrement à l'ancienne clé.
    pub migrated: bool,
}

/// Lit et valide un fichier de données.
pub fn read_database(path: &Path) -> AppResult<LoadedDatabase> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(validation(format!(
            "Le fichier fait {} Mio, au-delà de la limite de {} Mio.",
            metadata.len() / (1024 * 1024),
            MAX_FILE_BYTES / (1024 * 1024)
        )));
    }
    let bytes = fs::read(path)?;
    // Un fichier à l'ancienne clé doit être réécrit tout de suite avec la nouvelle.
    let legacy = crypto::is_legacy(&bytes);
    let contents = decode(&bytes)?;
    if contents.trim().is_empty() {
        return Ok(LoadedDatabase {
            database: Database::default(),
            migrated: false,
        });
    }
    let database: Database = serde_json::from_str(&contents)?;
    let migrated = legacy
        || database
            .customers
            .iter()
            .any(|customer| customer.id.trim().is_empty());
    Ok(LoadedDatabase {
        database: database.sanitized()?,
        migrated,
    })
}

/// Rend lisible le contenu d'un fichier de données, chiffré ou non.
///
/// Les deux formats sont acceptés : le fichier courant est chiffré, mais un export
/// lisible ou un fichier d'avant la migration doit rester importable.
pub fn decode(bytes: &[u8]) -> AppResult<String> {
    let plaintext = if crypto::is_encrypted(bytes) {
        crypto::open(bytes)?
    } else {
        bytes.to_vec()
    };
    String::from_utf8(plaintext)
        .map_err(|_| validation("Le fichier de données n'est pas du texte valide."))
}

/// Sérialise la base en JSON lisible.
pub fn to_plain_json(database: &Database) -> AppResult<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(database)?)
}

/// Sérialise la base puis la chiffre.
pub fn to_encrypted(database: &Database) -> AppResult<Vec<u8>> {
    crypto::seal(&to_plain_json(database)?)
}

/// Écrit la base via un fichier temporaire puis un renommage, afin qu'un fichier
/// partiellement écrit ne puisse jamais remplacer la version valide.
fn write_atomically(path: &Path, database: &Database) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = to_encrypted(database)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, &serialized)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temporary_store() -> (Store, PathBuf) {
        let mut directory = env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        directory.push(format!("bookmark-overlay-test-{stamp}"));
        let path = directory.join("data");
        let store = Store::load(path.clone()).expect("le dépôt se charge");
        (store, directory)
    }

    /// Clé de la fiche portant ce tenant, telle que le dépôt l'a attribuée.
    fn id_of(store: &Store, tenant: &str) -> String {
        store
            .snapshot()
            .expect("lecture")
            .customers
            .into_iter()
            .find(|entry| entry.has_tenant(tenant))
            .expect("fiche présente")
            .id
    }

    fn customer(tenant: &str, name: &str) -> Customer {
        Customer {
            id: String::new(),
            tenant: tenant.to_owned(),
            name: name.to_owned(),
            keywords: vec!["erp".to_owned()],
            folder_path: String::new(),
            devops_id: "1".to_owned(),
            github_id: "2".to_owned(),
            workspaces: Vec::new(),
            links: Vec::new(),
        }
    }

    #[test]
    fn create_then_read_roundtrips_through_the_file() {
        let (store, directory) = temporary_store();

        store.create(customer("acme", "Acme")).expect("création");
        let reloaded = Store::load(store.path().to_path_buf()).expect("rechargement");

        assert_eq!(reloaded.snapshot().expect("lecture").customers.len(), 1);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn is_fresh_only_reports_true_on_the_very_first_load() {
        let (store, directory) = temporary_store();
        assert!(store.is_fresh());

        let reloaded = Store::load(store.path().to_path_buf()).expect("rechargement");

        assert!(!reloaded.is_fresh());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn create_rejects_duplicate_tenant() {
        let (store, directory) = temporary_store();

        store.create(customer("acme", "Acme")).expect("création");
        let error = store.create(customer("ACME", "Acme bis"));

        assert!(matches!(error, Err(AppError::DuplicateTenant(_))));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_can_rename_the_tenant_without_changing_the_key() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");
        let id = id_of(&store, "acme");

        store
            .update(&id, customer("acme-2", "Acme"))
            .expect("mise à jour");

        assert_eq!(store.find(&id).expect("toujours là").tenant, "acme-2");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_customer_can_lose_its_tenant() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");
        let id = id_of(&store, "acme");

        store
            .update(&id, customer("", "Acme"))
            .expect("le tenant est facultatif");

        assert!(store.find(&id).expect("toujours là").tenant.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_refuses_a_tenant_already_taken() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création 1");
        store.create(customer("bolt", "Bolt")).expect("création 2");
        let bolt = id_of(&store, "bolt");

        let error = store.update(&bolt, customer("acme", "Bolt"));

        assert!(matches!(error, Err(AppError::DuplicateTenant(_))));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn delete_removes_the_entry() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");
        let id = id_of(&store, "acme");

        store.delete(&id).expect("suppression");

        assert!(store.snapshot().expect("lecture").customers.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn delete_unknown_key_is_reported() {
        let (store, directory) = temporary_store();
        assert!(matches!(store.delete("nope"), Err(AppError::NotFound(_))));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn customers_are_sorted_by_name() {
        let (store, directory) = temporary_store();
        store.create(customer("z", "Zebra")).expect("création 1");
        store.create(customer("a", "alpha")).expect("création 2");

        let names: Vec<String> = store
            .snapshot()
            .expect("lecture")
            .customers
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(names, vec!["alpha".to_owned(), "Zebra".to_owned()]);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn import_merge_adds_and_updates_without_losing_existing_entries() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");

        let incoming = Database {
            customers: vec![customer("acme", "Acme renommé"), customer("bolt", "Bolt")],
            settings: Settings::default(),
        };
        let report = store.import(incoming, ImportMode::Merge).expect("import");

        assert_eq!(report.added, 1);
        assert_eq!(report.updated, 1);
        assert_eq!(store.snapshot().expect("lecture").customers.len(), 2);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn import_replace_overwrites_everything() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");

        let incoming = Database {
            customers: vec![customer("bolt", "Bolt")],
            settings: Settings::default(),
        };
        store.import(incoming, ImportMode::Replace).expect("import");

        let tenants: Vec<String> = store
            .snapshot()
            .expect("lecture")
            .customers
            .into_iter()
            .map(|entry| entry.tenant)
            .collect();
        assert_eq!(tenants, vec!["bolt".to_owned()]);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn import_rejects_a_payload_with_duplicate_tenants() {
        let (store, directory) = temporary_store();

        let incoming = Database {
            customers: vec![customer("acme", "Acme"), customer("acme", "Acme bis")],
            settings: Settings::default(),
        };

        assert!(store.import(incoming, ImportMode::Replace).is_err());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn import_writes_a_backup_first() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");

        let report = store
            .import(Database::default(), ImportMode::Replace)
            .expect("import");

        assert!(Path::new(&report.backup_path).exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn keys_assigned_to_a_legacy_file_survive_a_reload() {
        let (store, directory) = temporary_store();
        let legacy = r#"{"customers":[{"tenant":"acme","name":"Acme"}]}"#;
        fs::write(store.path(), legacy).expect("écriture");

        let first = Store::load(store.path().to_path_buf()).expect("premier chargement");
        let id = first.snapshot().expect("lecture").customers[0].id.clone();
        let second = Store::load(store.path().to_path_buf()).expect("second chargement");

        assert!(!id.is_empty());
        assert_eq!(second.snapshot().expect("lecture").customers[0].id, id);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn the_stored_file_is_not_readable_as_text() {
        let (store, directory) = temporary_store();
        store
            .create(customer("acme", "Acme Industries"))
            .expect("création");

        let raw = fs::read(store.path()).expect("lecture brute");

        assert!(!raw.windows(15).any(|window| window == b"Acme Industries"));
        assert!(!raw.starts_with(b"{"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_legacy_plain_file_is_adopted_and_removed() {
        let (store, directory) = temporary_store();
        let legacy = directory.join("data.json");
        fs::write(
            &legacy,
            r#"{"customers":[{"tenant":"acme","name":"Acme"}]}"#,
        )
        .expect("écriture");
        fs::remove_file(store.path()).expect("retrait du fichier chiffré");

        let loaded = Store::load(store.path().to_path_buf()).expect("chargement");

        assert_eq!(loaded.snapshot().expect("lecture").customers.len(), 1);
        assert!(
            !legacy.exists(),
            "l'ancien fichier en clair doit disparaître"
        );
        assert!(
            directory.join("data.json.avant-chiffrement").is_file(),
            "une copie de sauvegarde doit rester"
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_corrupted_file_is_reported_and_never_overwritten() {
        let (store, directory) = temporary_store();
        store.create(customer("acme", "Acme")).expect("création");
        fs::write(store.path(), "{ ceci n'est pas du json").expect("écriture");

        assert!(Store::load(store.path().to_path_buf()).is_err());
        let _ = fs::remove_dir_all(directory);
    }
}
