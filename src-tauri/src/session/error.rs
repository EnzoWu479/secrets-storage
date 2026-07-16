//! Erros tipados da camada de sessões, seguros para cruzar o IPC.
//!
//! Nunca ecoam senha, chave ou plaintext (C-15). Falhas de autenticação de
//! qualquer origem (GMP, senha própria, adulteração de envelope) colapsam em
//! [`SessionError::Auth`] para não virar oráculo. Serializam como objeto
//! `{ "code": "...", ... }` (internally tagged) para o frontend discriminar.

use serde::Serialize;
use thiserror::Error;

use crate::crypto::CryptoError;

/// Erro de qualquer comando/operação da camada de sessões.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "code")]
pub enum SessionError {
    /// Operação exige o app desbloqueado (GMP), mas ele está bloqueado.
    #[error("app bloqueado")]
    AppLocked,

    /// A senha mestra global já foi criada (o keyring existe).
    #[error("senha global já existe")]
    AlreadyInitialized,

    /// Ainda não há senha mestra global (keyring ausente): fluxo de 1º uso.
    #[error("senha global não configurada")]
    NotInitialized,

    /// Sessão inexistente (ou id malformado). Genérico de propósito (VAULT-01 AC2).
    #[error("sessão não encontrada")]
    NotFound,

    /// Nome de sessão duplicado após normalização (VAULT-01 AC13).
    #[error("já existe uma sessão com esse nome")]
    DuplicateName,

    /// Senha/GMK incorreta ou envelope adulterado. Não revela qual.
    #[error("senha incorreta")]
    Auth,

    /// Senha abaixo do comprimento mínimo aceito pelo core.
    #[error("senha muito curta")]
    WeakPassword,

    /// Atraso progressivo ativo: aguarde antes de tentar de novo (VAULT-04 AC2).
    #[error("muitas tentativas; aguarde")]
    TooManyAttempts {
        /// Segundos até a próxima tentativa permitida.
        retry_after_secs: u64,
    },

    /// A operação exige uma sessão desbloqueada (ex.: `touch`).
    #[error("sessão bloqueada")]
    SessionLocked,

    /// Operação incompatível com o `auth_mode` da sessão
    /// (ex.: `unlock_session` numa sessão `global`).
    #[error("operação inválida para o modo desta sessão")]
    InvalidAuthMode,

    /// Falha de leitura/escrita no armazenamento local.
    #[error("falha de armazenamento")]
    Storage,

    /// Keyring/cofre corrompido, com magic inválido ou versão futura (fail-closed).
    #[error("cofre ou keyring incompatível ou corrompido")]
    CorruptOrIncompatible,

    /// Falha do CSPRNG ao gerar salt/nonce/chave.
    #[error("falha ao gerar aleatoriedade")]
    Random,
}

impl From<CryptoError> for SessionError {
    fn from(e: CryptoError) -> Self {
        match e {
            // Qualquer falha de autenticação vira um erro único (sem oráculo).
            CryptoError::Authentication => SessionError::Auth,
            // Formato/parâmetros: dado incompatível ou adulterado no disco.
            CryptoError::InvalidMagic
            | CryptoError::UnsupportedVersion
            | CryptoError::MalformedEnvelope
            | CryptoError::InvalidKdfParams
            | CryptoError::KeyDerivation => SessionError::CorruptOrIncompatible,
        }
    }
}

/// Resultado padrão da camada de sessões.
pub type Result<T> = core::result::Result<T, SessionError>;
