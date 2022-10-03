use std::ops::Deref;

use hex::{decode, encode};
use liot::noise::handshake::HandshakeXX;
use serde::{
    de::{Error, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Clone)]
struct HexString(Vec<u8>);

impl Deref for HexString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl AsRef<[u8]> for HexString {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for HexString {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String as Deserialize>::deserialize(d)?;
        let v = decode(&s)
            .map_err(|_e| D::Error::invalid_value(Unexpected::Str(&s), &"string in hex "))?;
        Ok(HexString(v))
    }
}

impl Serialize for HexString {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        encode(&self.0).serialize(s)
    }
}
#[derive(Serialize, Deserialize)]
struct TestMessage {
    payload: HexString,
    ciphertext: HexString,
}

#[derive(Serialize, Deserialize)]
struct TestUnit {
    protocol_name: String,
    init_prologue: HexString,
    resp_prologue: HexString,
    init_static: HexString,
    resp_static: HexString,
    init_ephemeral: HexString,
    resp_ephemeral: HexString,
    messages: Vec<TestMessage>,
}
#[test]
fn xx_test() {
    [
        include_str!("./vectors/snow.txt"),
        include_str!("./vectors/cacophony.txt"),
    ]
    .iter()
    .map(|s| serde_json::from_str::<TestUnit>(s).unwrap())
    .for_each(|v| {
        let mut init = HandshakeXX::init(
            v.init_ephemeral.0.as_slice().try_into().unwrap(),
            v.init_static.as_slice().try_into().unwrap(),
            v.init_prologue.0.as_slice(),
        );
        let mut resp = HandshakeXX::resp(
            v.resp_ephemeral.0.as_slice().try_into().unwrap(),
            v.resp_static.as_slice().try_into().unwrap(),
            v.resp_prologue.0.as_slice(),
        );
        let mut buf = [0; 1024];
        let mut msgs = v.messages.iter();

        let msg = msgs.next().unwrap();
        let len = init
            .write_message(msg.payload.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.ciphertext.0.as_slice());
        let len = resp
            .read_message(msg.ciphertext.0.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.payload.0.as_slice());

        let msg = msgs.next().unwrap();
        let len = resp
            .write_message(msg.payload.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.ciphertext.0.as_slice());
        let len = init
            .read_message(msg.ciphertext.0.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.payload.0.as_slice());

        let msg = msgs.next().unwrap();
        let len = init
            .write_message(msg.payload.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.ciphertext.0.as_slice());
        let len = resp
            .read_message(msg.ciphertext.0.as_slice(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..len], msg.payload.0.as_slice());

        let mut pair = [resp.upgrade().unwrap(), init.upgrade().unwrap()];
        for (count, msg) in msgs.enumerate() {
            let len = pair[count % 2]
                .write_message(msg.payload.0.as_slice(), &mut buf)
                .unwrap();
            assert_eq!(&buf[..len], msg.ciphertext.0.as_slice());
            let len = pair[(count + 1) % 2]
                .read_message(msg.ciphertext.0.as_slice(), &mut buf)
                .unwrap();
            assert_eq!(&buf[..len], msg.payload.0.as_slice());
        }
    });
}
