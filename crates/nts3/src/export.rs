/// Magic bytes identifying framework resource metadata in an ELF section.
pub const RESOURCE_MAGIC: [u8; 4] = *b"N3RS";
/// Current `.nts3_resources` record schema.
pub const RESOURCE_SCHEMA_VERSION: u16 = 1;

/// Compact, versioned metadata consumed by artifact inspection tools.
///
/// The record is packed so its serialized bytes are stable across host and
/// target architectures. Accessors copy fields by value and never expose
/// references to potentially unaligned storage.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ResourceRecord {
    magic: [u8; 4],
    schema_version: u16,
    record_size: u16,
    sdram_bytes: u32,
}

impl ResourceRecord {
    pub const fn new(sdram_bytes: u32) -> Self {
        Self {
            magic: RESOURCE_MAGIC,
            schema_version: RESOURCE_SCHEMA_VERSION,
            record_size: core::mem::size_of::<Self>() as u16,
            sdram_bytes,
        }
    }

    pub const fn magic(&self) -> [u8; 4] {
        self.magic
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn record_size(&self) -> u16 {
        self.record_size
    }

    pub const fn sdram_bytes(&self) -> u32 {
        self.sdram_bytes
    }
}

const _: () = {
    assert!(core::mem::size_of::<ResourceRecord>() == 12);
    assert!(core::mem::align_of::<ResourceRecord>() == 1);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_record_has_stable_versioned_bytes() {
        let record = ResourceRecord::new(1_100_000);
        assert_eq!(record.magic(), *b"N3RS");
        assert_eq!(record.schema_version(), 1);
        assert_eq!(record.record_size(), 12);
        assert_eq!(record.sdram_bytes(), 1_100_000);

        // SAFETY: `ResourceRecord` is packed, contains only initialized integer
        // and byte fields, and has no padding. The slice remains local.
        let bytes = unsafe {
            core::slice::from_raw_parts(
                core::ptr::addr_of!(record).cast::<u8>(),
                core::mem::size_of::<ResourceRecord>(),
            )
        };
        assert_eq!(&bytes[0..4], b"N3RS");
        assert_eq!(&bytes[4..6], &1_u16.to_ne_bytes());
        assert_eq!(&bytes[6..8], &12_u16.to_ne_bytes());
        assert_eq!(&bytes[8..12], &1_100_000_u32.to_ne_bytes());
    }
}
