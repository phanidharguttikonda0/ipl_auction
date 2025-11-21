use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SdpPayload {
    #[serde(rename = "type")]
    pub sdp_type: String, // basically offer or answer, rust doesn't support type keyword, so kept it as sdp_type
    pub sdp: String,
}

#[derive(Serialize, Deserialize,Debug)]
pub struct IceCandidatePayload {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]  // <-- IMPORTANT
pub enum SignalingMessage {
    #[serde(rename = "offer")]
    Offer {
        from: i32,
        to: i32,
        payload: SdpPayload,
    },

    #[serde(rename = "answer")]
    Answer {
        from: i32,
        to: i32,
        payload: SdpPayload,
    },

    #[serde(rename = "ice-candidate")]
    IceCandidate {
        from: i32,
        to: i32,
        payload: IceCandidatePayload,
    },
}

/*
    as front-end sends the json in the following way 
    {
  "type": "offer",
  "from": 1,
  "to": 2,
  "payload": {
    "type": "offer",
    "sdp": "v=0 ... "
  }
}
-> so we are going to use the enum with renaming 

*/