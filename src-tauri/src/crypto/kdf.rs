//! Derivação de chave a partir de senha (Argon2id): senha → KEK.
//!
//! `derive_kek` transforma uma senha/GMP em uma chave de envelopamento de 32 bytes
//! ([`Key32`]). Salt e parâmetros vêm **por parâmetro** (injetáveis), o que torna os
//! vetores de teste determinísticos sem gerar aleatoriedade dentro do núcleo.
//!
//! Os parâmetros numéricos são **candidatos** (`⚠️ PT-01`) — não finais enquanto o
//! modelo de ameaças estiver em revisão (D-05). [`KdfParams::validate`] rejeita valores
//! fora dos limites defensivos **antes** de qualquer alocação (edge case anti-DoS).

use argon2::{Algorithm, Argon2, Params, Version};

use crate::crypto::{CryptoError, Key32, Result};

/// Limite inferior de memória: 8 MiB. Abaixo disso o Argon2id fica fraco demais.
pub const MIN_MEM_KIB: u32 = 8 * 1024;
/// Limite superior de memória: 4 GiB. Teto anti-DoS — recusado antes de alocar.
pub const MAX_MEM_KIB: u32 = 4 * 1024 * 1024;
/// Mínimo de iterações (passes).
pub const MIN_ITERS: u32 = 1;
/// Máximo de iterações — teto anti-DoS.
pub const MAX_ITERS: u32 = 32;
/// Mínimo de paralelismo (lanes).
pub const MIN_PARALLELISM: u32 = 1;
/// Máximo de paralelismo — teto anti-DoS.
pub const MAX_PARALLELISM: u32 = 16;

/// Parâmetros do Argon2id. Valores **candidatos** (`⚠️ PT-01`), não finais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    /// Custo de memória, em KiB.
    pub mem_kib: u32,
    /// Número de iterações (passes de tempo).
    pub iters: u32,
    /// Grau de paralelismo (lanes).
    pub parallelism: u32,
}

impl KdfParams {
    /// Parâmetros candidatos do design (`⚠️ PT-01`): 64 MiB, 3 iterações, 1 lane.
    /// **Não** são a configuração final; servem para destravar a fatia.
    pub const CANDIDATE: Self = Self {
        mem_kib: 64 * 1024,
        iters: 3,
        parallelism: 1,
    };

    /// Rejeita parâmetros fora dos limites defensivos **sem** alocar memória.
    ///
    /// Protege contra tanto configurações fracas (mínimo seguro) quanto pedidos
    /// de memória absurdos que travariam o processo (teto anti-DoS).
    pub fn validate(&self) -> Result<()> {
        if !(MIN_MEM_KIB..=MAX_MEM_KIB).contains(&self.mem_kib)
            || !(MIN_ITERS..=MAX_ITERS).contains(&self.iters)
            || !(MIN_PARALLELISM..=MAX_PARALLELISM).contains(&self.parallelism)
        {
            return Err(CryptoError::InvalidKdfParams);
        }
        Ok(())
    }
}

/// Deriva a KEK de 32 bytes a partir de `password` e `salt` via Argon2id.
///
/// Determinístico para `(password, salt, params)` fixos. Valida `params` antes de
/// alocar a memória do KDF. Não gera aleatoriedade: o salt é responsabilidade do chamador.
pub fn derive_kek(password: &[u8], salt: &[u8], params: KdfParams) -> Result<Key32> {
    params.validate()?;

    let argon_params = Params::new(params.mem_kib, params.iters, params.parallelism, Some(32))
        .map_err(|_| CryptoError::InvalidKdfParams)?;

    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut out = [0u8; 32];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(|_| CryptoError::KeyDerivation)?;

    Ok(Key32::from_bytes(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Params reduzidos para teste rápido (no limite mínimo de memória).
    const TEST_PARAMS: KdfParams = KdfParams {
        mem_kib: MIN_MEM_KIB,
        iters: 1,
        parallelism: 1,
    };

    #[test]
    fn deriva_deterministicamente() {
        let salt = b"salt-de-16-bytes";
        let a = derive_kek(b"senha-correta", salt, TEST_PARAMS).unwrap();
        let b = derive_kek(b"senha-correta", salt, TEST_PARAMS).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn salts_diferentes_geram_keks_diferentes() {
        let a = derive_kek(b"senha", b"salt-de-16-byteA", TEST_PARAMS).unwrap();
        let b = derive_kek(b"senha", b"salt-de-16-byteB", TEST_PARAMS).unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn senhas_diferentes_geram_keks_diferentes() {
        let salt = b"salt-de-16-bytes";
        let a = derive_kek(b"senha-A", salt, TEST_PARAMS).unwrap();
        let b = derive_kek(b"senha-B", salt, TEST_PARAMS).unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn memoria_acima_do_teto_e_rejeitada_antes_de_alocar() {
        // MAX + 1 nunca deve ser alocado: validate() corta antes.
        let params = KdfParams {
            mem_kib: MAX_MEM_KIB + 1,
            iters: 1,
            parallelism: 1,
        };
        assert!(matches!(
            derive_kek(b"senha", b"salt-de-16-bytes", params),
            Err(CryptoError::InvalidKdfParams)
        ));
    }

    #[test]
    fn memoria_abaixo_do_minimo_e_rejeitada() {
        let params = KdfParams {
            mem_kib: MIN_MEM_KIB - 1,
            iters: 1,
            parallelism: 1,
        };
        assert!(matches!(
            params.validate(),
            Err(CryptoError::InvalidKdfParams)
        ));
    }

    #[test]
    fn iters_e_parallelism_fora_do_limite_sao_rejeitados() {
        let iters_alto = KdfParams {
            iters: MAX_ITERS + 1,
            ..TEST_PARAMS
        };
        let par_zero = KdfParams {
            parallelism: 0,
            ..TEST_PARAMS
        };
        assert!(iters_alto.validate().is_err());
        assert!(par_zero.validate().is_err());
    }

    #[test]
    fn candidato_esta_dentro_dos_limites() {
        assert!(KdfParams::CANDIDATE.validate().is_ok());
    }
}
