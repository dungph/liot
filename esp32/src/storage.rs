use std::{collections::BTreeSet, sync::Arc};

use base58::ToBase58;
use embedded_svc::storage::{RawStorage, StorageBase};
use esp_idf_svc::{nvs::EspDefaultNvs, nvs_storage::EspNvsStorage};
use once_cell::sync::Lazy;
use postcard::to_allocvec;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

static DEFAULT_NVS: Lazy<Arc<EspDefaultNvs>> =
    Lazy::new(|| Arc::new(EspDefaultNvs::new().unwrap()));
pub fn take() -> Arc<EspDefaultNvs> {
    DEFAULT_NVS.clone()
}
pub fn get<T: DeserializeOwned>(key: &str) -> anyhow::Result<Option<T>> {
    let mut buf = [0u8; 128];
    let nvs = EspNvsStorage::new_default(take(), "data", true)?;
    Ok(if let Some((buf, _len)) = nvs.get_raw(key, &mut buf)? {
        Some(postcard::from_bytes(buf)?)
    } else {
        None
    })
}
pub fn set<T: Serialize>(key: &str, data: T) -> anyhow::Result<()> {
    let buf = postcard::to_allocvec(&data)?;
    let mut nvs = EspNvsStorage::new_default(take(), "data", true)?;
    nvs.put_raw(key, &buf)?;
    Ok(())
}
#[derive(Serialize, Deserialize, Default, Debug)]
struct PeerRole {
    is_manager: bool,
    is_subscriber: bool,
    is_controller: bool,
}

fn get_rfid_list() -> anyhow::Result<BTreeSet<Vec<u8>>> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let mut buf = [0u8; 320];
    if nvs.contains("rfid")? {
        let (data, _len) = nvs.get_raw("rfid", &mut buf)?.unwrap_or_default();
        Ok(postcard::from_bytes(data)?)
    } else {
        let default_data: BTreeSet<Vec<u8>> = BTreeSet::new();
        let data: Vec<u8> = postcard::to_allocvec(&default_data)?;
        nvs.put_raw("rfid", &data)?;
        Ok(BTreeSet::new())
    }
}
pub fn set_rfid_list(list: BTreeSet<Vec<u8>>) -> anyhow::Result<()> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let data: Vec<u8> = postcard::to_allocvec(&list)?;
    nvs.put_raw("rfid", &data)?;
    Ok(())
}
fn get_subscriber_list() -> anyhow::Result<BTreeSet<Vec<u8>>> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let mut buf = [0u8; 320];
    if nvs.contains("subscriber")? {
        let (data, _len) = nvs.get_raw("subscriber", &mut buf)?.unwrap_or_default();
        Ok(postcard::from_bytes(data)?)
    } else {
        let default_data: BTreeSet<Vec<u8>> = BTreeSet::new();
        let data: Vec<u8> = postcard::to_allocvec(&default_data)?;
        nvs.put_raw("subscriber", &data)?;
        Ok(BTreeSet::new())
    }
}
fn set_subscriber_list(list: BTreeSet<Vec<u8>>) -> anyhow::Result<()> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let data: Vec<u8> = postcard::to_allocvec(&list)?;
    nvs.put_raw("subscriber", &data)?;
    Ok(())
}
pub fn get_controller_list() -> anyhow::Result<BTreeSet<Vec<u8>>> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let mut buf = [0u8; 320];
    if nvs.contains("controller")? {
        let (data, _len) = nvs.get_raw("controller", &mut buf)?.unwrap_or_default();
        Ok(postcard::from_bytes(data)?)
    } else {
        let default_data: BTreeSet<Vec<u8>> = BTreeSet::new();
        let data: Vec<u8> = postcard::to_allocvec(&default_data)?;
        nvs.put_raw("controller", &data)?;
        Ok(BTreeSet::new())
    }
}
pub fn set_controller_list(list: BTreeSet<Vec<u8>>) -> anyhow::Result<()> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let data: Vec<u8> = postcard::to_allocvec(&list)?;
    nvs.put_raw("controller", &data)?;
    Ok(())
}
fn get_manager_list() -> anyhow::Result<BTreeSet<Vec<u8>>> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let mut buf = [0u8; 320];
    if nvs.contains("manager")? {
        let (data, _len) = nvs.get_raw("manager", &mut buf)?.unwrap_or_default();
        Ok(postcard::from_bytes(data)?)
    } else {
        let default_data: BTreeSet<Vec<u8>> = BTreeSet::new();
        let data: Vec<u8> = postcard::to_allocvec(&default_data)?;
        nvs.put_raw("manager", &data)?;
        Ok(default_data)
    }
}
fn set_manager_list(list: BTreeSet<Vec<u8>>) -> anyhow::Result<()> {
    let mut nvs = EspNvsStorage::new_default(take(), "role", true)?;
    let data: Vec<u8> = postcard::to_allocvec(&list)?;
    nvs.put_raw("manager", &data)?;
    Ok(())
}

pub fn is_rfid(peer: &[u8]) -> anyhow::Result<bool> {
    if peer == b"mqtt" || peer == b"internal" {
        return Ok(true);
    }
    Ok(get_rfid_list()?.contains(&peer.to_vec()))
}
pub fn add_rfid(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_rfid_list()?;
    list.insert(peer.to_vec());
    set_rfid_list(list)?;
    Ok(())
}
pub fn rm_rfid(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_rfid_list()?;
    list.remove(peer);
    set_rfid_list(list)?;
    Ok(())
}
pub fn is_controller(peer: &[u8]) -> anyhow::Result<bool> {
    if peer == b"mqtt" || peer == b"internal" {
        return Ok(true);
    }
    let list = get_controller_list()?;
    let peer = peer.to_vec();
    Ok(list.contains(&peer))
}
pub fn add_controller(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_controller_list()?;
    list.insert(peer.to_vec());
    Ok(set_controller_list(list)?)
}
pub fn rm_controller(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_controller_list()?;
    list.remove(peer);
    set_controller_list(list)?;
    Ok(())
}
pub fn is_subscriber(peer: &[u8]) -> anyhow::Result<bool> {
    if peer == b"mqtt" || peer == b"internal" {
        return Ok(true);
    }
    Ok(get_subscriber_list()?.contains(&peer.to_vec()))
}
pub fn add_subscriber(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_subscriber_list()?;
    list.insert(peer.to_vec());
    Ok(set_subscriber_list(list)?)
}
pub fn rm_subscriber(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_subscriber_list()?;
    list.remove(peer);
    set_subscriber_list(list)?;
    Ok(())
}
pub fn is_manager(peer: &[u8]) -> anyhow::Result<bool> {
    if peer == b"mqtt" {
        return Ok(true);
    }
    Ok(get_manager_list()?.contains(&peer.to_vec()))
}
pub fn add_manager(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_manager_list()?;
    list.insert(peer.to_vec());
    Ok(set_manager_list(list)?)
}
pub fn rm_manager(peer: &[u8]) -> anyhow::Result<()> {
    let mut list = get_manager_list()?;
    list.remove(peer);
    set_manager_list(list)?;
    Ok(())
}

//pub fn set_ssid(ssid: heapless::String<32>) -> anyhow::Result<()> {
//    let mut nvs = EspNvsStorage::new_default(take(), "wifi", true)?;
//    nvs.put_raw("ssid", ssid.as_bytes())?;
//    Ok(())
//}
//pub fn get_ssid() -> anyhow::Result<heapless::String<32>> {
//    let nvs = EspNvsStorage::new_default(take(), "wifi", false)?;
//    let mut buf = [0; 32];
//    let (s, _len) = nvs.get_raw("ssid", &mut buf).unwrap().unwrap_or_default();
//    Ok(heapless::String::from(String::from_utf8_lossy(s).as_ref()))
//}
//pub fn set_pwd(pwd: heapless::String<64>) -> anyhow::Result<()> {
//    let mut nvs = EspNvsStorage::new_default(take(), "wifi", true)?;
//    nvs.put_raw("pwd", pwd.as_bytes())?;
//    Ok(())
//}
//pub fn get_pwd() -> anyhow::Result<heapless::String<64>> {
//    let nvs = EspNvsStorage::new_default(take(), "wifi", false)?;
//    let mut buf = [0; 64];
//    let (s, _len) = nvs.get_raw("pwd", &mut buf)?.unwrap_or_default();
//    Ok(heapless::String::from(String::from_utf8_lossy(s).as_ref()))
//}

pub fn private_key() -> anyhow::Result<[u8; 32]> {
    let mut nvs = EspNvsStorage::new_default(take(), "private key", true)?;
    let mut key = rand::random::<[u8; 32]>();
    if let Some((private_key, _len)) = nvs.get_raw("key", &mut key)? {
        Ok(private_key.try_into().unwrap())
    } else {
        nvs.put_raw("key", &key)?;
        Ok(key)
    }
}
pub fn public_key() -> anyhow::Result<[u8; 32]> {
    let lrivate = private_key()?;
    Ok(x25519_dalek::x25519(
        lrivate,
        x25519_dalek::X25519_BASEPOINT_BYTES,
    ))
}
