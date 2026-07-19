use cgmath::Vector3;

/// Benchmark-owned 4x4 column-major matrix.
///
/// The implementation is kept mechanically aligned with
/// `native/flecs_c/math.hpp` so every adapter executes the same scalar
/// inversion and vector-transform algorithm.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchmarkMatrix {
    elements: [f32; 16],
}

impl BenchmarkMatrix {
    const BENCHMARK_COSINE_BITS: u32 = 0x3EB9_86F5;
    const BENCHMARK_SINE_BITS: u32 = 0x3F6E_9A1D;

    pub fn identity() -> Self {
        let mut elements = [0.0; 16];
        elements[0] = 1.0;
        elements[5] = 1.0;
        elements[10] = 1.0;
        elements[15] = 1.0;
        Self { elements }
    }

    pub fn from_scale(scale: f32) -> Self {
        let mut matrix = Self::identity();
        matrix.elements[0] = scale;
        matrix.elements[5] = scale;
        matrix.elements[10] = scale;
        matrix
    }

    pub fn rotation_x(radians: f32) -> Self {
        Self::rotation_x_from_cosine_sine(radians.cos(), radians.sin())
    }

    pub fn benchmark_rotation_x() -> Self {
        Self::rotation_x_from_cosine_sine(
            f32::from_bits(Self::BENCHMARK_COSINE_BITS),
            f32::from_bits(Self::BENCHMARK_SINE_BITS),
        )
    }

    fn rotation_x_from_cosine_sine(cosine: f32, sine: f32) -> Self {
        let mut matrix = Self::identity();
        matrix.elements[5] = cosine;
        matrix.elements[6] = sine;
        matrix.elements[9] = -sine;
        matrix.elements[10] = cosine;
        matrix
    }

    #[inline(always)]
    pub fn inverse(self) -> Self {
        self.try_inverse()
            .expect("benchmark matrix should remain invertible")
    }

    #[inline(always)]
    pub fn transform_vector(self, vector: Vector3<f32>) -> Vector3<f32> {
        let m = &self.elements;
        Vector3::new(
            m[0] * vector.x + m[4] * vector.y + m[8] * vector.z,
            m[1] * vector.x + m[5] * vector.y + m[9] * vector.z,
            m[2] * vector.x + m[6] * vector.y + m[10] * vector.z,
        )
    }

    #[inline(always)]
    fn try_inverse(self) -> Option<Self> {
        let m = &self.elements;
        let mut inverse = [0.0; 16];

        inverse[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        inverse[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        inverse[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        inverse[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        inverse[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
            - m[9] * m[3] * m[14]
            - m[13] * m[2] * m[11]
            + m[13] * m[3] * m[10];
        inverse[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
            + m[8] * m[3] * m[14]
            + m[12] * m[2] * m[11]
            - m[12] * m[3] * m[10];
        inverse[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
            - m[8] * m[3] * m[13]
            - m[12] * m[1] * m[11]
            + m[12] * m[3] * m[9];
        inverse[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
            + m[8] * m[2] * m[13]
            + m[12] * m[1] * m[10]
            - m[12] * m[2] * m[9];
        inverse[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
            + m[5] * m[3] * m[14]
            + m[13] * m[2] * m[7]
            - m[13] * m[3] * m[6];
        inverse[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
            - m[4] * m[3] * m[14]
            - m[12] * m[2] * m[7]
            + m[12] * m[3] * m[6];
        inverse[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
            + m[4] * m[3] * m[13]
            + m[12] * m[1] * m[7]
            - m[12] * m[3] * m[5];
        inverse[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
            - m[4] * m[2] * m[13]
            - m[12] * m[1] * m[6]
            + m[12] * m[2] * m[5];
        inverse[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
            - m[5] * m[3] * m[10]
            - m[9] * m[2] * m[7]
            + m[9] * m[3] * m[6];
        inverse[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
            + m[4] * m[3] * m[10]
            + m[8] * m[2] * m[7]
            - m[8] * m[3] * m[6];
        inverse[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
            - m[4] * m[3] * m[9]
            - m[8] * m[1] * m[7]
            + m[8] * m[3] * m[5];
        inverse[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
            + m[4] * m[2] * m[9]
            + m[8] * m[1] * m[6]
            - m[8] * m[2] * m[5];

        let determinant =
            m[0] * inverse[0] + m[1] * inverse[4] + m[2] * inverse[8] + m[3] * inverse[12];
        if determinant == 0.0 {
            return None;
        }

        let reciprocal = 1.0 / determinant;
        for value in &mut inverse {
            *value *= reciprocal;
        }
        Some(Self { elements: inverse })
    }
}

impl Default for BenchmarkMatrix {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::BenchmarkMatrix;
    use cgmath::Vector3;

    #[test]
    fn layout_matches_native_matrix() {
        assert_eq!(std::mem::size_of::<BenchmarkMatrix>(), 16 * 4);
        assert_eq!(std::mem::align_of::<BenchmarkMatrix>(), 4);
    }

    #[test]
    fn inverse_round_trip_restores_rotation() {
        let matrix = BenchmarkMatrix::rotation_x(1.2);
        let restored = matrix.inverse().inverse();
        for (actual, expected) in restored.elements.iter().zip(matrix.elements) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn inverse_rotation_transforms_a_vector() {
        let matrix = BenchmarkMatrix::benchmark_rotation_x();
        assert_eq!(
            matrix.elements[5].to_bits(),
            BenchmarkMatrix::BENCHMARK_COSINE_BITS
        );
        assert_eq!(
            matrix.elements[6].to_bits(),
            BenchmarkMatrix::BENCHMARK_SINE_BITS
        );
        let output = matrix
            .inverse()
            .transform_vector(Vector3::new(1.0, 2.0, 3.0));
        let cosine = f32::from_bits(BenchmarkMatrix::BENCHMARK_COSINE_BITS);
        let sine = f32::from_bits(BenchmarkMatrix::BENCHMARK_SINE_BITS);
        assert!((output.x - 1.0).abs() < 1.0e-5);
        assert!((output.y - (2.0 * cosine + 3.0 * sine)).abs() < 1.0e-5);
        assert!((output.z - (-2.0 * sine + 3.0 * cosine)).abs() < 1.0e-5);
    }
}
