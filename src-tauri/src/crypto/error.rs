//! Erros tipados do núcleo criptográfico.
//!
//! `Display` nunca inclui bytes de chave, senha ou plaintext: mensagens são
//! genéricas o suficiente para log seguro (fronteira do design crypto-format).

use thiserror::Error;

/// Erro de qualquer operação do módulo `crypto`.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Parâmetros de KDF fora dos limites defensivos (mínimo seguro / máximo anti-DoS).
    /// Retornado **antes** de alocar a memória pedida.
    #[error("parâmetros de KDF inválidos ou fora dos limites permitidos")]
    InvalidKdfParams,

    /// Falha ao derivar material de chave (erro interno do KDF/HKDF).
    #[error("falha na derivação de chave")]
    KeyDerivation,

    /// Falha de autenticação do AEAD: unwrap/decrypt de dado adulterado,
    /// senha/GMK errada, ou AAD divergente. Não revela qual.
    #[error("falha de autenticação: dado adulterado ou chave incorreta")]
    Authentication,

    /// Magic identifier do envelope não corresponde ao esperado.
    #[error("identificador de formato inválido")]
    InvalidMagic,

    /// Versão do formato superior à suportada: fail-closed, não interpreta.
    #[error("versão de formato não suportada (superior à corrente)")]
    UnsupportedVersion,

    /// Serialização/desserialização CBOR malformada ou envelope truncado.
    #[error("envelope malformado ou truncado")]
    MalformedEnvelope,
}

/// Resultado padrão do núcleo criptográfico.
pub type Result<T> = core::result::Result<T, CryptoError>;
