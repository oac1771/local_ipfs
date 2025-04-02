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
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum IpfsPinResponse {
        Ls(IpfsPinLsResponse),
        Add(IpfsPinAddResponse),
    }

    impl From<IpfsPinLsResponse> for IpfsPinResponse {
        fn from(value: IpfsPinLsResponse) -> Self {
            Self::Ls(value)
        }
    }

    impl From<IpfsPinAddResponse> for IpfsPinResponse {
        fn from(value: IpfsPinAddResponse) -> Self {
            Self::Add(value)
        }
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

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsPinAddResponse {
        #[serde(alias = "Keys")]
        pub keys: Value,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsAddResponse {
        #[serde(alias = "Hash")]
        pub hash: String,
        #[serde(alias = "Name")]
        pub name: String,
    }
}
