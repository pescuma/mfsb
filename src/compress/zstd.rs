use super::*;
use ::paste::paste;

pub struct ZstdCompressor {
    level: i32,
}

impl CompressorFactory for ZstdCompressor {
    type Type = ZstdCompressor;

    fn name() -> &'static str {
        "zstd"
    }

    fn new() -> Self::Type {
        ZstdCompressor { level: 0 }
    }
}

macro_rules! factory {
    ($i:literal) => {
        paste! {
            pub struct [<Zstd $i Compressor>] {}

            impl CompressorFactory for [<Zstd $i Compressor>] {
                type Type = ZstdCompressor;

                fn name() -> &'static str {
                    concat!("zstd-level-", stringify!($i))
                }

                fn new() -> Self::Type {
                    ZstdCompressor { level: $i }
                }
            }
        }
    };
}

factory!(1);
factory!(2);
factory!(3);
factory!(4);
factory!(5);
factory!(6);
factory!(7);
factory!(8);
factory!(9);

impl Compressor for ZstdCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let result = ::zstd::stream::encode_all(data, self.level)?;
        Ok(result)
    }
}
