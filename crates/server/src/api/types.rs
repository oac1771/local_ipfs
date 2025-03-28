use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct Pong {
    pub response: String,
}

pub mod ipfs {
    use super::*;

    #[allow(non_camel_case_types)]
    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum PinAction {
        ls,
        add,
        // rm
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsIdResponse {
        #[serde(alias = "ID")]
        pub id: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsPinLsResponse {
        #[serde(alias = "Keys")]
        pub keys: Value,
    }
}
