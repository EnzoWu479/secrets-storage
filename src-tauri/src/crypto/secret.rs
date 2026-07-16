//! Material de chave zeroizável compartilhado pelo núcleo criptográfico.
//!
//! `Key32` guarda 32 bytes de segredo (KEK, gKEK, GMK, root_key, subchaves).
//! É zeroizado no `Drop` e não expõe `Debug`/`Display` para não vazar o
//! conteúdo em logs. Nenhum valor de `Key32` cruza o IPC para a WebView.

use zeroize::Zeroize;

/// Chave simétrica de 32 bytes, apagada da memória ao ser descartada.
///
/// Deliberadamente **não** implementa `Debug`, `Display`, `Clone` nem
/// serialização: material de chave só existe em memória do core e some no drop.
pub struct Key32([u8; 32]);

impl Key32 {
    /// Constrói a partir de 32 bytes já materializados (KDF, HKDF ou CSPRNG).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Acesso somente-leitura aos bytes, para uso como chave de AEAD/HKDF.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for Key32 {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constroi_e_le_os_mesmos_bytes() {
        let bytes = [7u8; 32];
        let key = Key32::from_bytes(bytes);
        assert_eq!(key.as_bytes(), &bytes);
    }
}
