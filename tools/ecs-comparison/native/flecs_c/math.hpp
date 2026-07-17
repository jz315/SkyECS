#pragma once

#include <array>
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <type_traits>

namespace sky_ecs_bench {

struct Vec3 {
    float x;
    float y;
    float z;
};

class Mat4 {
public:
    static Mat4 identity() noexcept {
        Mat4 matrix;
        matrix[0] = 1.0f;
        matrix[5] = 1.0f;
        matrix[10] = 1.0f;
        matrix[15] = 1.0f;
        return matrix;
    }

    static Mat4 rotation_x(float radians) noexcept {
        Mat4 matrix = identity();
        const float cosine = std::cos(radians);
        const float sine = std::sin(radians);
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

    Mat4 inverse() const {
        Mat4 result;
        if (!try_inverse(result)) {
            std::abort();
        }
        return result;
    }

    Vec3 transform_vector(const Vec3& vector) const noexcept {
        return {
            (*this)[0] * vector.x + (*this)[4] * vector.y + (*this)[8] * vector.z,
            (*this)[1] * vector.x + (*this)[5] * vector.y + (*this)[9] * vector.z,
            (*this)[2] * vector.x + (*this)[6] * vector.y + (*this)[10] * vector.z,
        };
    }

private:
    bool try_inverse(Mat4& destination) const noexcept {
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

} // namespace sky_ecs_bench
