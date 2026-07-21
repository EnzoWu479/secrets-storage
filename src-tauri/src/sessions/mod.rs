//! Ciclo de vida de sessões locais e desbloqueio por senha mestra.
//!
//! Esta fatia entrega o `SessionManager` de produção que substitui o fake
//! `crate::secrets::session_access::FakeSessionAccess`, fornecendo a evidência
//! de liberação do gate externo G1 de `secret-management`.

pub mod app_lock;
pub mod attempts;
pub mod commands;
pub mod manager;
pub mod model;
pub mod registry;
