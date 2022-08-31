use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;

use common::BinarySerializable;
use fastfield_codecs::bitpacked::{BitpackedCodec, BitpackedReader};
use fastfield_codecs::blockwise_linear::{BlockwiseLinearCodec, BlockwiseLinearReader};
use fastfield_codecs::linear::{LinearCodec, LinearReader};
use fastfield_codecs::{Column, FastFieldCodec, FastFieldCodecType};

use super::gcd::open_gcd_from_bytes;
use super::FastValue;
use crate::directory::{CompositeFile, Directory, FileSlice, OwnedBytes, RamDirectory, WritePtr};
use crate::error::DataCorruption;
use crate::fastfield::{CompositeFastFieldSerializer, FastFieldsWriter, GCDReader};
use crate::schema::{Schema, FAST};

#[derive(Clone)]
/// DynamicFastFieldReader wraps different readers to access
/// the various encoded fastfield data
pub enum DynamicFastFieldReader<Item: FastValue> {
    /// Bitpacked compressed fastfield data.
    Bitpacked(FastFieldReaderCodecWrapper<Item, BitpackedReader>),
    /// Linear interpolated values + bitpacked
    Linear(FastFieldReaderCodecWrapper<Item, LinearReader>),
    /// Blockwise linear interpolated values + bitpacked
    BlockwiseLinear(FastFieldReaderCodecWrapper<Item, BlockwiseLinearReader>),

    /// GCD and Bitpacked compressed fastfield data.
    BitpackedGCD(FastFieldReaderCodecWrapper<Item, GCDReader<BitpackedReader>>),
    /// GCD and Linear interpolated values + bitpacked
    LinearGCD(FastFieldReaderCodecWrapper<Item, GCDReader<LinearReader>>),
    /// GCD and Blockwise linear interpolated values + bitpacked
    BlockwiseLinearGCD(FastFieldReaderCodecWrapper<Item, GCDReader<BlockwiseLinearReader>>),
}

impl<Item: FastValue> DynamicFastFieldReader<Item> {
    /// Returns correct the reader wrapped in the `DynamicFastFieldReader` enum for the data.
    pub fn open_from_id(
        mut bytes: OwnedBytes,
        codec_type: FastFieldCodecType,
    ) -> crate::Result<DynamicFastFieldReader<Item>> {
        let reader =
            match codec_type {
                FastFieldCodecType::Bitpacked => DynamicFastFieldReader::Bitpacked(
                    BitpackedCodec::open_from_bytes(bytes)?.into(),
                ),
                FastFieldCodecType::Linear => {
                    DynamicFastFieldReader::Linear(LinearCodec::open_from_bytes(bytes)?.into())
                }
                FastFieldCodecType::BlockwiseLinear => DynamicFastFieldReader::BlockwiseLinear(
                    BlockwiseLinearCodec::open_from_bytes(bytes)?.into(),
                ),
                FastFieldCodecType::Gcd => {
                    let codec_type = FastFieldCodecType::deserialize(&mut bytes)?;
                    match codec_type {
                        FastFieldCodecType::Bitpacked => DynamicFastFieldReader::BitpackedGCD(
                            open_gcd_from_bytes::<BitpackedCodec>(bytes)?.into(),
                        ),
                        FastFieldCodecType::Linear => DynamicFastFieldReader::LinearGCD(
                            open_gcd_from_bytes::<LinearCodec>(bytes)?.into(),
                        ),
                        FastFieldCodecType::BlockwiseLinear => {
                            DynamicFastFieldReader::BlockwiseLinearGCD(
                                open_gcd_from_bytes::<BlockwiseLinearCodec>(bytes)?.into(),
                            )
                        }
                        FastFieldCodecType::Gcd => return Err(DataCorruption::comment_only(
                            "Gcd codec wrapped into another gcd codec. This combination is not \
                             allowed.",
                        )
                        .into()),
                    }
                }
            };
        Ok(reader)
    }

    /// Returns correct the reader wrapped in the `DynamicFastFieldReader` enum for the data.
    pub fn open(file: FileSlice) -> crate::Result<DynamicFastFieldReader<Item>> {
        let mut bytes = file.read_bytes()?;
        let codec_type = FastFieldCodecType::deserialize(&mut bytes)?;
        Self::open_from_id(bytes, codec_type)
    }
}

impl<Item: FastValue> Column<Item> for DynamicFastFieldReader<Item> {
    #[inline]
    fn get_val(&self, idx: u64) -> Item {
        match self {
            Self::Bitpacked(reader) => reader.get_val(idx),
            Self::Linear(reader) => reader.get_val(idx),
            Self::BlockwiseLinear(reader) => reader.get_val(idx),
            Self::BitpackedGCD(reader) => reader.get_val(idx),
            Self::LinearGCD(reader) => reader.get_val(idx),
            Self::BlockwiseLinearGCD(reader) => reader.get_val(idx),
        }
    }
    fn min_value(&self) -> Item {
        match self {
            Self::Bitpacked(reader) => reader.min_value(),
            Self::Linear(reader) => reader.min_value(),
            Self::BlockwiseLinear(reader) => reader.min_value(),
            Self::BitpackedGCD(reader) => reader.min_value(),
            Self::LinearGCD(reader) => reader.min_value(),
            Self::BlockwiseLinearGCD(reader) => reader.min_value(),
        }
    }
    fn max_value(&self) -> Item {
        match self {
            Self::Bitpacked(reader) => reader.max_value(),
            Self::Linear(reader) => reader.max_value(),
            Self::BlockwiseLinear(reader) => reader.max_value(),
            Self::BitpackedGCD(reader) => reader.max_value(),
            Self::LinearGCD(reader) => reader.max_value(),
            Self::BlockwiseLinearGCD(reader) => reader.max_value(),
        }
    }

    fn num_vals(&self) -> u64 {
        match self {
            Self::Bitpacked(reader) => reader.num_vals(),
            Self::Linear(reader) => reader.num_vals(),
            Self::BlockwiseLinear(reader) => reader.num_vals(),
            Self::BitpackedGCD(reader) => reader.num_vals(),
            Self::LinearGCD(reader) => reader.num_vals(),
            Self::BlockwiseLinearGCD(reader) => reader.num_vals(),
        }
    }
}

/// Wrapper for accessing a fastfield.
///
/// Holds the data and the codec to the read the data.
#[derive(Clone)]
pub struct FastFieldReaderCodecWrapper<Item: FastValue, CodecReader> {
    reader: CodecReader,
    _phantom: PhantomData<Item>,
}

impl<Item: FastValue, CodecReader> From<CodecReader>
    for FastFieldReaderCodecWrapper<Item, CodecReader>
{
    fn from(reader: CodecReader) -> Self {
        FastFieldReaderCodecWrapper {
            reader,
            _phantom: PhantomData,
        }
    }
}

impl<Item: FastValue, D: Column> FastFieldReaderCodecWrapper<Item, D> {
    #[inline]
    pub(crate) fn get_u64(&self, idx: u64) -> Item {
        let data = self.reader.get_val(idx);
        Item::from_u64(data)
    }
}

impl<Item: FastValue, C: Column + Clone> Column<Item> for FastFieldReaderCodecWrapper<Item, C> {
    /// Return the value associated to the given document.
    ///
    /// This accessor should return as fast as possible.
    ///
    /// # Panics
    ///
    /// May panic if `doc` is greater than the segment
    // `maxdoc`.
    fn get_val(&self, idx: u64) -> Item {
        self.get_u64(idx)
    }

    /// Returns the minimum value for this fast field.
    ///
    /// The max value does not take in account of possible
    /// deleted document, and should be considered as an upper bound
    /// of the actual maximum value.
    fn min_value(&self) -> Item {
        Item::from_u64(self.reader.min_value())
    }

    /// Returns the maximum value for this fast field.
    ///
    /// The max value does not take in account of possible
    /// deleted document, and should be considered as an upper bound
    /// of the actual maximum value.
    fn max_value(&self) -> Item {
        Item::from_u64(self.reader.max_value())
    }

    fn num_vals(&self) -> u64 {
        self.reader.num_vals()
    }
}

impl<Item: FastValue> From<Vec<Item>> for DynamicFastFieldReader<Item> {
    fn from(vals: Vec<Item>) -> DynamicFastFieldReader<Item> {
        let mut schema_builder = Schema::builder();
        let field = schema_builder.add_u64_field("field", FAST);
        let schema = schema_builder.build();
        let path = Path::new("__dummy__");
        let directory: RamDirectory = RamDirectory::create();
        {
            let write: WritePtr = directory
                .open_write(path)
                .expect("With a RamDirectory, this should never fail.");
            let mut serializer = CompositeFastFieldSerializer::from_write(write)
                .expect("With a RamDirectory, this should never fail.");
            let mut fast_field_writers = FastFieldsWriter::from_schema(&schema);
            {
                let fast_field_writer = fast_field_writers
                    .get_field_writer_mut(field)
                    .expect("With a RamDirectory, this should never fail.");
                for val in vals {
                    fast_field_writer.add_val(val.to_u64());
                }
            }
            fast_field_writers
                .serialize(&mut serializer, &HashMap::new(), None)
                .unwrap();
            serializer.close().unwrap();
        }

        let file = directory.open_read(path).expect("Failed to open the file");
        let composite_file = CompositeFile::open(&file).expect("Failed to read the composite file");
        let field_file = composite_file
            .open_read(field)
            .expect("File component not found");
        DynamicFastFieldReader::open(field_file).unwrap()
    }
}
