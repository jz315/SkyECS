use criterion::BatchSize;

/// Construction inputs become worlds larger than one MiB inside the measured
/// closure. Retain only a small number per batch so allocator retention does
/// not become an accidental part of the operation contract.
pub fn construction_batch_size() -> BatchSize {
    BatchSize::LargeInput
}
