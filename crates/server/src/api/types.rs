use serde::{Deserialize, Serialize};

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
        rm
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsIdResponse {
        #[serde(alias = "ID")]
        pub id: String,
    }
}
