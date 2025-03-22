use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Pong {
    pub response: String,
}

pub mod ipfs {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum PinAction {
        Ls
    }

    impl TryFrom<String> for PinAction {
        type Error = &'static str;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            match value.as_str() {
                "ls" => Ok(Self::Ls),
                _ => Err("foo")
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct IpfsIdResponse {
        #[serde(alias = "ID")]
        pub id: String,
    }
}
