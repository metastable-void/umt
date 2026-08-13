//! Realization records (UMT-3.2 part VII).
//!
//! Only the provenance identifier exists so far. Residual taxonomy,
//! optimization outcomes, realization records, and the compiled performance
//! plan belong to a later stage; the identifier is defined now because
//! UMT-3.2 section 0.6.1 requires every real-valued observation to reference
//! its provenance from the moment it can be constructed.

pub mod provenance;
