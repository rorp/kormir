use gloo_utils::format::JsValueSerdeExt;
use kormir::bitcoin::secp256k1::rand;
use kormir::error::Error;
use kormir::storage::{OracleEventData, Storage};
use kormir::{OracleAnnouncement, Signature};
use rexie::{ObjectStore, Rexie, TransactionMode};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::JsValue;

const DATABASE_NAME: &str = "kormir";
const OBJECT_STORE_NAME: &str = "oracle";
pub const MNEMONIC_KEY: &str = "mnemonic";
const NONCE_INDEX_KEY: &str = "nonce_index";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i32,
    announcement_signature: Vec<u8>,
    oracle_event: Vec<u8>,
    pub name: String,
    pub is_enum: bool,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNonce {
    pub id: i32,
    pub event_id: i32,
    pub index: i32,
    nonce: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    pub outcome: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct IndexedDb {
    current_index: Arc<AtomicU32>,
    rexie: Rexie,
}

impl IndexedDb {
    async fn build_indexed_db() -> Rexie {
        Rexie::builder(DATABASE_NAME)
            .version(1)
            .add_object_store(ObjectStore::new(OBJECT_STORE_NAME))
            .build()
            .await
            .unwrap()
    }

    pub async fn new() -> Result<Self, Error> {
        let rexie = Self::build_indexed_db().await;

        let tx = rexie
            .transaction(&[OBJECT_STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|_| Error::StorageFailure)?;
        let store = tx
            .store(OBJECT_STORE_NAME)
            .map_err(|_| Error::StorageFailure)?;

        // get current nonce index from the database
        let js = store
            .get(&JsValue::from_serde(NONCE_INDEX_KEY).unwrap())
            .await
            .map_err(|_| Error::StorageFailure)?;
        let index: Option<u32> = js.into_serde().map_err(|_| Error::StorageFailure)?;

        tx.done().await.map_err(|_| Error::StorageFailure)?;

        Ok(Self {
            current_index: Arc::new(AtomicU32::new(index.unwrap_or(0))),
            rexie,
        })
    }

    pub async fn save_to_indexed_db<K: Serialize, V: Serialize>(
        &self,
        key: K,
        value: V,
    ) -> Result<(), Error> {
        let tx = self
            .rexie
            .transaction(&[OBJECT_STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|_| Error::StorageFailure)?;
        let store = tx
            .store(OBJECT_STORE_NAME)
            .map_err(|_| Error::StorageFailure)?;
        store
            .put(
                &JsValue::from_serde(&value).map_err(|_| Error::StorageFailure)?,
                Some(&JsValue::from_serde(&key).map_err(|_| Error::StorageFailure)?),
            )
            .await
            .map_err(|_| Error::StorageFailure)?;
        tx.done().await.map_err(|_| Error::StorageFailure)?;
        Ok(())
    }

    pub async fn get_from_indexed_db<K: Serialize, V>(&self, key: K) -> Result<Option<V>, Error>
    where
        V: for<'a> serde::de::Deserialize<'a>,
    {
        let tx = self
            .rexie
            .transaction(&[OBJECT_STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|_| Error::StorageFailure)?;
        let store = tx
            .store(OBJECT_STORE_NAME)
            .map_err(|_| Error::StorageFailure)?;
        let js = store
            .get(&JsValue::from_serde(&key).map_err(|_| Error::StorageFailure)?)
            .await
            .map_err(|_| Error::StorageFailure)?;
        tx.done().await.map_err(|_| Error::StorageFailure)?;

        let value: Option<V> = js.into_serde().map_err(|_| Error::StorageFailure)?;
        Ok(value)
    }

    pub async fn add_announcement_event_id(&self, id: u32, event_id: String) -> Result<(), Error> {
        let tx = self
            .rexie
            .transaction(&[OBJECT_STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|_| Error::StorageFailure)?;
        let store = tx
            .store(OBJECT_STORE_NAME)
            .map_err(|_| Error::StorageFailure)?;
        let js = store
            .get(&JsValue::from_serde(&id).map_err(|_| Error::StorageFailure)?)
            .await
            .map_err(|_| Error::StorageFailure)?;
        let mut event: OracleEventData = js.into_serde().map_err(|_| Error::StorageFailure)?;
        event.announcement_event_id = Some(event_id);
        store
            .put(
                &JsValue::from_serde(&event).map_err(|_| Error::StorageFailure)?,
                Some(&JsValue::from_serde(&id).map_err(|_| Error::StorageFailure)?),
            )
            .await
            .map_err(|_| Error::StorageFailure)?;
        tx.done().await.map_err(|_| Error::StorageFailure)?;
        Ok(())
    }

    pub async fn add_attestation_event_id(&self, id: u32, event_id: String) -> Result<(), Error> {
        let tx = self
            .rexie
            .transaction(&[OBJECT_STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|_| Error::StorageFailure)?;
        let store = tx
            .store(OBJECT_STORE_NAME)
            .map_err(|_| Error::StorageFailure)?;
        let js = store
            .get(&JsValue::from_serde(&id).map_err(|_| Error::StorageFailure)?)
            .await
            .map_err(|_| Error::StorageFailure)?;
        let mut event: OracleEventData = js.into_serde().map_err(|_| Error::StorageFailure)?;
        event.attestation_event_id = Some(event_id);
        store
            .put(
                &JsValue::from_serde(&event).map_err(|_| Error::StorageFailure)?,
                Some(&JsValue::from_serde(&id).map_err(|_| Error::StorageFailure)?),
            )
            .await
            .map_err(|_| Error::StorageFailure)?;
        tx.done().await.map_err(|_| Error::StorageFailure)?;
        Ok(())
    }
}

impl Storage for IndexedDb {
    async fn get_next_nonce_indexes(&self, num: usize) -> Result<Vec<u32>, Error> {
        let mut current_index = self.current_index.fetch_add(num as u32, Ordering::Relaxed);
        let mut indexes = Vec::with_capacity(num);
        for _ in 0..num {
            indexes.push(current_index);
            current_index += 1;
        }
        self.save_to_indexed_db(NONCE_INDEX_KEY, current_index)
            .await?;
        Ok(indexes)
    }

    async fn save_announcement(
        &self,
        announcement: OracleAnnouncement,
        indexes: Vec<u32>,
    ) -> Result<u32, Error> {
        // generate random id
        let id = rand::random::<u32>();
        let event = OracleEventData {
            announcement,
            indexes,
            signatures: Vec::new(),
            announcement_event_id: None,
            attestation_event_id: None,
        };

        self.save_to_indexed_db(id, event).await?;

        Ok(id)
    }

    async fn save_signatures(
        &self,
        id: u32,
        sigs: Vec<Signature>,
    ) -> Result<OracleEventData, Error> {
        let mut event = self.get_event(id).await?.ok_or(Error::NotFound)?;
        if !event.signatures.is_empty() {
            return Err(Error::EventAlreadySigned);
        }

        event.signatures = sigs;
        self.save_to_indexed_db(id, &event).await?;

        Ok(event)
    }

    async fn get_event(&self, id: u32) -> Result<Option<OracleEventData>, Error> {
        let event: Option<OracleEventData> = self.get_from_indexed_db(id).await?;
        Ok(event)
    }
}