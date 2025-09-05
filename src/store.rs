use crate::error::{Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zstd::{Decoder, Encoder};

const MAGIC_ID: &[u8] = b"OGMA";
const VERSION: u16 = 3;

#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(i32);

/// The compression level to use when writing to disk.
/// Compression is done with zstandard and any valid compression level can be used.
/// For simplicity, a set of pre-defined levels are provided.
impl CompressionLevel {
    /// The absolute fastest compression level with the lowest ratio. Maps to -7 in zstd.
    pub const FASTEST: CompressionLevel = CompressionLevel(Self::ZSTD_MIN);

    /// The absolute slowest compression level with the highest ratio. Maps to 22 in zstd.
    /// Typically only suitable for archival purposes as the memory and CPU overhead are significant
    /// for comparatively small gains over faster compression levels.
    pub const SMALLEST_SIZE: CompressionLevel = CompressionLevel(Self::ZSTD_MAX);

    /// The default compression level. Maps to 3 in zstd.
    /// This is the recommended level for most use cases as it prioritizes speed with a reasonable ratio.
    pub const DEFAULT: CompressionLevel = Self::FAST;

    /// Faster compression at the cost of higher ratios. Maps to 3 in zstd.
    /// This is the recommended level for most use cases as it prioritizes speed with a reasonable ratio.
    pub const FAST: CompressionLevel = CompressionLevel(3);

    /// Balances compression speed and ratio.
    /// More suitable for cases where size is just as important as speed. Maps to 6 in zstd.
    pub const BALANCED: CompressionLevel = CompressionLevel(6);

    /// Slower compression speeds but higher compression ratios. Maps to 9 in zstd.
    /// Recommended for cases where a smaller size is more important than speed.
    pub const OPTIMAL: CompressionLevel = CompressionLevel(9);
    pub const ZSTD_MIN: i32 = -7;
    pub const ZSTD_MAX: i32 = 22;

    /// Use a custom compression level.
    /// Any valid zstd level is allowed.
    /// Values outside the range of -7 (very fast) and 22 (very slow) are clamped.
    pub fn new(level: i32) -> Self {
        Self(level.clamp(Self::ZSTD_MIN, Self::ZSTD_MAX))
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone)]
pub struct StoreOptions {
    pub path: PathBuf,
    pub compression_level: CompressionLevel,
}

impl StoreOptions {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            compression_level: CompressionLevel::DEFAULT,
        }
    }

    pub fn set_compression_level(&mut self, level: CompressionLevel) {
        self.compression_level = level;
    }

    pub fn with_compression_level(mut self, level: CompressionLevel) -> Self {
        self.set_compression_level(level);
        self
    }
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Store<K, V>
where
    K: Eq + Hash
{
    map: HashMap<K, V>,

    #[serde(skip)]
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

            let mut dec = Decoder::new(file)?;
            let store: Store<K, V> = rmp_serde::decode::from_read(&mut dec)?;

            Ok(Self { options, ..store})
        }
    }

    pub fn save(&self) -> Result<()> {
        let temp_path = self.options.path.with_extension("ogma.tmp");
        let mut file = File::create(&temp_path)?;

        file.write_all(MAGIC_ID)?;
        file.write_u16::<LittleEndian>(VERSION)?;

        let mut enc = Encoder::new(file, self.options.compression_level.0)?;
        rmp_serde::encode::write(&mut enc, self)?;
        let mut file = enc.finish()?;

        file.sync_all()?;
        file.flush()?;
        drop(file);

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
