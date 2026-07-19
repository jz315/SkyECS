#pragma once

#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <type_traits>

namespace sky_ecs_bench {

#if defined(_MSC_VER)
#define SKY_BENCH_ALWAYS_INLINE __forceinline
#else
#define SKY_BENCH_ALWAYS_INLINE inline __attribute__((always_inline))
#endif

struct Vec3 {
    float x;
    float y;
    float z;
};

inline std::uint64_t add_vector_checksum(
    std::uint64_t checksum,
    const Vec3& vector) noexcept {
    std::uint32_t x = 0;
    std::uint32_t y = 0;
    std::uint32_t z = 0;
    std::memcpy(&x, &vector.x, sizeof(x));
    std::memcpy(&y, &vector.y, sizeof(y));
    std::memcpy(&z, &vector.z, sizeof(z));
    return checksum + x + y + z;
}

class Mat4 {
public:
    static constexpr std::uint32_t BENCHMARK_COSINE_BITS = 0x3EB986F5;
    static constexpr std::uint32_t BENCHMARK_SINE_BITS = 0x3F6E9A1D;

    static Mat4 identity() noexcept {
        Mat4 matrix;
        matrix[0] = 1.0f;
        matrix[5] = 1.0f;
        matrix[10] = 1.0f;
        matrix[15] = 1.0f;
        return matrix;
    }

    static Mat4 rotation_x(float radians) noexcept {
        return rotation_x_from_cosine_sine(
            std::cos(radians),
            std::sin(radians));
    }

    static Mat4 benchmark_rotation_x() noexcept {
        float cosine = 0.0f;
        float sine = 0.0f;
        const std::uint32_t cosine_bits = BENCHMARK_COSINE_BITS;
        const std::uint32_t sine_bits = BENCHMARK_SINE_BITS;
        std::memcpy(&cosine, &cosine_bits, sizeof(cosine));
        std::memcpy(&sine, &sine_bits, sizeof(sine));
        return rotation_x_from_cosine_sine(cosine, sine);
    }

    static Mat4 rotation_x_from_cosine_sine(
        float cosine,
        float sine) noexcept {
        Mat4 matrix = identity();
        matrix[5] = cosine;
        matrix[6] = sine;
        matrix[9] = -sine;
        matrix[10] = cosine;
        return matrix;
    }

    float& operator[](std::size_t index) noexcept {
        return elements_[index];
    }

    const float& operator[](std::size_t index) const noexcept {
        return elements_[index];
    }

    auto begin() noexcept { return elements_.begin(); }
    auto end() noexcept { return elements_.end(); }
    auto begin() const noexcept { return elements_.begin(); }
    auto end() const noexcept { return elements_.end(); }

    SKY_BENCH_ALWAYS_INLINE Mat4 inverse() const {
        Mat4 result;
        if (!try_inverse(result)) {
            std::abort();
        }
        return result;
    }

    SKY_BENCH_ALWAYS_INLINE Vec3 transform_vector(
        const Vec3& vector) const noexcept {
        return {
            (*this)[0] * vector.x + (*this)[4] * vector.y + (*this)[8] * vector.z,
            (*this)[1] * vector.x + (*this)[5] * vector.y + (*this)[9] * vector.z,
            (*this)[2] * vector.x + (*this)[6] * vector.y + (*this)[10] * vector.z,
        };
    }

private:
    SKY_BENCH_ALWAYS_INLINE bool try_inverse(
        Mat4& destination) const noexcept {
        const float* m = elements_.data();
        std::array<float, 16> inverse{};

        inverse[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] -
            m[9] * m[6] * m[15] + m[9] * m[7] * m[14] +
            m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
        inverse[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] +
            m[8] * m[6] * m[15] - m[8] * m[7] * m[14] -
            m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
        inverse[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] -
            m[8] * m[5] * m[15] + m[8] * m[7] * m[13] +
            m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
        inverse[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] +
            m[8] * m[5] * m[14] - m[8] * m[6] * m[13] -
            m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
        inverse[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] +
            m[9] * m[2] * m[15] - m[9] * m[3] * m[14] -
            m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
        inverse[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] -
            m[8] * m[2] * m[15] + m[8] * m[3] * m[14] +
            m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
        inverse[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] +
            m[8] * m[1] * m[15] - m[8] * m[3] * m[13] -
            m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
        inverse[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] -
            m[8] * m[1] * m[14] + m[8] * m[2] * m[13] +
            m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
        inverse[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] -
            m[5] * m[2] * m[15] + m[5] * m[3] * m[14] +
            m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
        inverse[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] +
            m[4] * m[2] * m[15] - m[4] * m[3] * m[14] -
            m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
        inverse[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] -
            m[4] * m[1] * m[15] + m[4] * m[3] * m[13] +
            m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
        inverse[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] +
            m[4] * m[1] * m[14] - m[4] * m[2] * m[13] -
            m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
        inverse[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] +
            m[5] * m[2] * m[11] - m[5] * m[3] * m[10] -
            m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
        inverse[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] -
            m[4] * m[2] * m[11] + m[4] * m[3] * m[10] +
            m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
        inverse[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] +
            m[4] * m[1] * m[11] - m[4] * m[3] * m[9] -
            m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
        inverse[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] -
            m[4] * m[1] * m[10] + m[4] * m[2] * m[9] +
            m[8] * m[1] * m[6] - m[8] * m[2] * m[5];

        const float determinant = m[0] * inverse[0] + m[1] * inverse[4] +
            m[2] * inverse[8] + m[3] * inverse[12];
        if (determinant == 0.0f) {
            return false;
        }

        const float reciprocal = 1.0f / determinant;
        for (std::size_t index = 0; index < elements_.size(); ++index) {
            destination[index] = inverse[index] * reciprocal;
        }
        return true;
    }

    std::array<float, 16> elements_{};
};

static_assert(sizeof(Mat4) == sizeof(float) * 16);
static_assert(std::is_trivially_copyable_v<Mat4>);
static_assert(std::is_standard_layout_v<Mat4>);

#undef SKY_BENCH_ALWAYS_INLINE

} // namespace sky_ecs_bench
