pub mod assertions;
pub mod constants;
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

pub use solana_program;

solana_program::declare_id!("8PtjrGvKNeZt2vCmRkSPGjss7TAFhvxux2N8r67UMKBx");

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "YourWallet Stake Pool",
    project_url: "https://github.com/yourwalletio/yw-stake-pool",
    contacts: "email:hello@yourwallet.tr",
    policy: "https://github.com/yourwalletio/yw-stake-pool/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/yourwalletio/yw-stake-pool",
    auditors: "See audit/SECURITY_AUDIT.md"
}
