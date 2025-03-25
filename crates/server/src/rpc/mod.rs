mod error;
pub mod ipfs;
pub mod ping;

pub enum Module {
    Ping,
    Ipfs,
}
