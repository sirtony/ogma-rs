use crate::error::{Error, Result};
use brotli2::read::BrotliDecoder;
use brotli2::write::BrotliEncoder;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC_ID: &[u8] = b"OGMA";
const VERSION: u16 = 2;
const BROTLI_MIN_LEVEL: u32 = 0;
const BROTLI_MAX_LEVEL: u32 = 11;
const BROTLI_DEFAULT_LEVEL: u32 = 5;

#[derive(Debug, Clone)]
pub struct StoreOptions {
    pub path: PathBuf,
    pub compression_level: u32,
}

impl StoreOptions {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            compression_level: BROTLI_DEFAULT_LEVEL,
        }
    }

    pub fn set_compression_level(&mut self, level: u32) {
        self.compression_level = level.clamp(BROTLI_MIN_LEVEL, BROTLI_MAX_LEVEL);
    }

    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.set_compression_level(level);
        self
    }
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self::new("./store.ogma").with_compression_level(BROTLI_DEFAULT_LEVEL)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Record<K, V>
where
    K: Eq + Hash,
{
    #[serde(rename = "Key")]
    pub key: K,
    #[serde(rename = "Value")]
    pub value: V,
}

#[derive(Debug, Serialize, Deserialize)]
struct Document<K, V>
where
    K: Eq + Hash,
{
    // just contains the records for now, but other fields like metadata
    // could be added later
    #[serde(rename = "Store")]
    pub store: Vec<Record<K, V>>,
}

#[derive(Debug)]
pub struct Store<K, V>
where
    K: Eq + Hash + Serialize + for<'de> Deserialize<'de>,
    V: Serialize + for<'de> Deserialize<'de>,
{
    map: HashMap<K, V>,
    options: StoreOptions,
}

impl<K, V> Store<K, V>
where
    K: Eq + Hash + Serialize + for<'de> Deserialize<'de>,
    V: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(options: StoreOptions) -> Self {
        Self {
            map: HashMap::new(),
            options,
        }
    }

    pub fn open(options: StoreOptions) -> Result<Self> {
        if !options.path.exists() || !options.path.is_file() {
            Ok(Self {
                map: HashMap::new(),
                options,
            })
        } else {
            let mut file = File::open(&options.path)?;
            let mut magic_id = [0u8; 4];
            file.read_exact(&mut magic_id)?;
            if magic_id != MAGIC_ID {
                return Err(Error::InvalidFile);
            }

            let version = file.read_u16::<LittleEndian>()?;
            if version != VERSION {
                return Err(Error::WrongVersion {
                    expected: VERSION,
                    actual: version,
                });
            }

            let brotli = BrotliDecoder::new(file);
            let doc: Document<K, V> = serde_json::from_reader(brotli)?;

            Ok(Self {
                map: doc
                    .store
                    .into_iter()
                    .fold(HashMap::new(), |mut map, record| {
                        map.insert(record.key, record.value);
                        map
                    }),
                options,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let temp_path = self.options.path.with_extension("ogma.tmp");
        let mut file = File::create(&temp_path)?;

        file.write_all(MAGIC_ID)?;
        file.write_u16::<LittleEndian>(VERSION)?;

        let doc: Document<&K, &V> = Document {
            store: self
                .map
                .iter()
                .map(|(key, value)| Record { key, value })
                .collect(),
        };

        let mut brotli = BrotliEncoder::new(file, self.options.compression_level);
        serde_json::to_writer(&mut brotli, &doc)?;
        brotli.flush()?;

        drop(brotli);

        std::fs::rename(&temp_path, &self.options.path)?;

        Ok(())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    pub fn set(&mut self, key: K, value: V) -> Option<V> {
        self.map.insert(key, value)
    }

    pub fn delete(&mut self, key: &K) -> Option<V> {
        self.map.remove(key)
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.map.keys()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.map.values()
    }

    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<'_, K, V> {
        self.map.values_mut()
    }
}
