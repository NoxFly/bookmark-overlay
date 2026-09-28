//! Type d'erreur unique de l'application, sérialisable vers le front.

use std::fmt::Display;

/// Erreurs remontées par les commandes Tauri et la couche de persistance.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Échec d'une opération sur le système de fichiers.
    #[error("erreur d'accès au fichier : {0}")]
    Io(#[from] std::io::Error),

    /// Le fichier de données n'est pas un JSON conforme au schéma attendu.
    #[error("données JSON invalides : {0}")]
    Json(#[from] serde_json::Error),

    /// Une donnée fournie par le front ne respecte pas les invariants métier.
    #[error("{0}")]
    Validation(String),

    /// Aucun client ne porte ce tenant.
    #[error("aucun client ne correspond au tenant « {0} »")]
    NotFound(String),

    /// Le tenant est la clé primaire : il ne peut pas être dupliqué.
    #[error("le tenant « {0} » est déjà utilisé par un autre client")]
    DuplicateTenant(String),

    /// Le verrou protégeant l'état partagé a été empoisonné par un panic antérieur.
    #[error("état interne corrompu, redémarre l'application")]
    PoisonedState,

    /// Erreur propagée par le runtime Tauri (fenêtre, écran, événement...).
    #[error("erreur système : {0}")]
    Tauri(#[from] tauri::Error),

    /// Le lanceur externe (navigateur, explorateur) n'a pas pu être démarré.
    #[error("{0}")]
    Launch(String),

    /// La recherche ou l'installation d'une mise à jour a échoué.
    #[error("mise à jour impossible : {0}")]
    Update(String),
}

impl From<ureq::Error> for AppError {
    fn from(error: ureq::Error) -> Self {
        Self::Update(error.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Raccourci de retour pour toutes les fonctions faillibles de l'application.
pub type AppResult<T> = Result<T, AppError>;

/// Construit une erreur de validation à partir de n'importe quel message affichable.
pub fn validation<M: Display>(message: M) -> AppError {
    AppError::Validation(message.to_string())
}
