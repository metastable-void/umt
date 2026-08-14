//! Exact integer matrices.
//!
//! Storage is row-major and every entry is a [`Z`], so no operation can
//! overflow. Matrices are used for temperament mappings, lattice bases, and
//! the transformation matrices of the normal forms in
//! [`crate::algebra::normal_form`].

use alloc::vec::Vec;

use num_traits::Zero;

use crate::algebra::Z;
use crate::error::MatrixError;

/// A matrix of exact integers.
///
/// UMT layer: L1/L2, exact. Equality is presentation equality: same shape and
/// same entries. Two matrices that encode the same lattice or the same
/// homomorphism up to a change of basis are *not* equal; that question is
/// answered by the normal forms and by the lattice types built on them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntMatrix {
    rows: usize,
    cols: usize,
    /// Row-major, `rows * cols` entries.
    data: Vec<Z>,
}

impl IntMatrix {
    /// Builds a matrix from row-major data.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::DataLength`] if `data.len() != rows * cols`.
    pub fn new(rows: usize, cols: usize, data: Vec<Z>) -> Result<Self, MatrixError> {
        let expected = rows.checked_mul(cols).ok_or(MatrixError::DataLength {
            expected: usize::MAX,
            found: data.len(),
        })?;
        if data.len() != expected {
            return Err(MatrixError::DataLength {
                expected,
                found: data.len(),
            });
        }
        Ok(Self { rows, cols, data })
    }

    /// Builds a matrix from a sequence of rows.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::RaggedRows`] if the rows differ in length.
    ///
    /// # Examples
    ///
    /// ```
    /// use umt::algebra::matrix::IntMatrix;
    ///
    /// let m = IntMatrix::from_rows([[12i64, 19, 28]])?;
    /// assert_eq!((m.rows(), m.cols()), (1, 3));
    /// # Ok::<(), umt::error::MatrixError>(())
    /// ```
    pub fn from_rows<R, C, T>(rows: R) -> Result<Self, MatrixError>
    where
        R: IntoIterator<Item = C>,
        C: IntoIterator<Item = T>,
        T: Into<Z>,
    {
        let mut data = Vec::new();
        let mut row_count = 0usize;
        let mut cols: Option<usize> = None;
        for row in rows {
            let before = data.len();
            data.extend(row.into_iter().map(Into::into));
            let width = data.len() - before;
            match cols {
                None => cols = Some(width),
                Some(expected) if expected != width => {
                    return Err(MatrixError::RaggedRows {
                        expected,
                        found: width,
                    });
                }
                Some(_) => {}
            }
            row_count += 1;
        }
        let cols = cols.unwrap_or(0);
        Ok(Self {
            rows: row_count,
            cols,
            data,
        })
    }

    /// A `rows` by `cols` matrix of zeros.
    #[must_use]
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: (0..rows * cols).map(|_| Z::zero()).collect(),
        }
    }

    /// The `size` by `size` identity matrix.
    #[must_use]
    pub fn identity(size: usize) -> Self {
        let mut matrix = Self::zeros(size, size);
        for i in 0..size {
            matrix.data[i * size + i] = Z::from(1);
        }
        matrix
    }

    /// Number of rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// The entry at `(row, col)`, or `None` if out of bounds.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> Option<&Z> {
        if row < self.rows && col < self.cols {
            Some(&self.data[row * self.cols + col])
        } else {
            None
        }
    }

    /// The entry at `(row, col)`.
    ///
    /// # Panics
    ///
    /// Panics if the indices are out of bounds, like ordinary slice indexing.
    /// Use [`IntMatrix::get`] for externally supplied indices.
    #[must_use]
    pub fn at(&self, row: usize, col: usize) -> &Z {
        assert!(
            row < self.rows && col < self.cols,
            "index ({row}, {col}) out of bounds for a {}x{} matrix",
            self.rows,
            self.cols
        );
        &self.data[row * self.cols + col]
    }

    /// Replaces the entry at `(row, col)`.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::IndexOutOfBounds`] if the indices are out of
    /// range.
    pub fn set(&mut self, row: usize, col: usize, value: Z) -> Result<(), MatrixError> {
        if row >= self.rows || col >= self.cols {
            return Err(MatrixError::IndexOutOfBounds { row, col });
        }
        self.data[row * self.cols + col] = value;
        Ok(())
    }

    /// The entries of one row.
    #[must_use]
    pub fn row(&self, row: usize) -> Option<&[Z]> {
        if row < self.rows {
            Some(&self.data[row * self.cols..(row + 1) * self.cols])
        } else {
            None
        }
    }

    /// The entries of one column, copied.
    #[must_use]
    pub fn column(&self, col: usize) -> Option<Vec<Z>> {
        if col < self.cols {
            Some(
                (0..self.rows)
                    .map(|row| self.at(row, col).clone())
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Whether every entry is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.data.iter().all(Zero::is_zero)
    }

    /// The same matrix with its rows in reverse order.
    ///
    /// Used to run a normal form with the coordinate order reversed, which
    /// changes which coordinates its pivots prefer to eliminate.
    #[must_use]
    pub fn reverse_rows(&self) -> Self {
        let mut out = Self::zeros(self.rows, self.cols);
        for row in 0..self.rows {
            let source = self.rows - 1 - row;
            for col in 0..self.cols {
                out.data[row * self.cols + col] = self.at(source, col).clone();
            }
        }
        out
    }

    /// The transpose.
    #[must_use]
    pub fn transpose(&self) -> Self {
        let mut out = Self::zeros(self.cols, self.rows);
        for row in 0..self.rows {
            for col in 0..self.cols {
                out.data[col * self.rows + row] = self.at(row, col).clone();
            }
        }
        out
    }

    /// Matrix product `self * other`.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::DimensionMismatch`] if the inner dimensions
    /// differ.
    pub fn multiply(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.cols != other.rows {
            return Err(MatrixError::DimensionMismatch {
                left: self.cols,
                right: other.rows,
            });
        }
        let mut out = Self::zeros(self.rows, other.cols);
        for row in 0..self.rows {
            for inner in 0..self.cols {
                let left = self.at(row, inner);
                if left.is_zero() {
                    continue;
                }
                for col in 0..other.cols {
                    let product = left * other.at(inner, col);
                    out.data[row * other.cols + col] += product;
                }
            }
        }
        Ok(out)
    }

    /// Applies the matrix to a column vector.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::DimensionMismatch`] if the vector length differs
    /// from the column count.
    pub fn apply(&self, vector: &[Z]) -> Result<Vec<Z>, MatrixError> {
        if vector.len() != self.cols {
            return Err(MatrixError::DimensionMismatch {
                left: self.cols,
                right: vector.len(),
            });
        }
        Ok((0..self.rows)
            .map(|row| {
                (0..self.cols)
                    .map(|col| self.at(row, col) * &vector[col])
                    .sum()
            })
            .collect())
    }

    /// A new matrix built from the given columns, in the given order.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::IndexOutOfBounds`] if a column index is out of
    /// range.
    pub fn select_columns(&self, columns: &[usize]) -> Result<Self, MatrixError> {
        let mut out = Self::zeros(self.rows, columns.len());
        for (target, source) in columns.iter().enumerate() {
            if *source >= self.cols {
                return Err(MatrixError::IndexOutOfBounds {
                    row: 0,
                    col: *source,
                });
            }
            for row in 0..self.rows {
                out.data[row * columns.len() + target] = self.at(row, *source).clone();
            }
        }
        Ok(out)
    }

    pub(crate) fn swap_rows(&mut self, first: usize, second: usize) {
        if first == second {
            return;
        }
        for col in 0..self.cols {
            self.data
                .swap(first * self.cols + col, second * self.cols + col);
        }
    }

    pub(crate) fn swap_columns(&mut self, first: usize, second: usize) {
        if first == second {
            return;
        }
        for row in 0..self.rows {
            self.data
                .swap(row * self.cols + first, row * self.cols + second);
        }
    }

    /// `row[target] += factor * row[source]`.
    pub(crate) fn add_scaled_row(&mut self, target: usize, source: usize, factor: &Z) {
        if factor.is_zero() {
            return;
        }
        for col in 0..self.cols {
            let addend = self.at(source, col) * factor;
            self.data[target * self.cols + col] += addend;
        }
    }

    /// `column[target] += factor * column[source]`.
    pub(crate) fn add_scaled_column(&mut self, target: usize, source: usize, factor: &Z) {
        if factor.is_zero() {
            return;
        }
        for row in 0..self.rows {
            let addend = self.at(row, source) * factor;
            self.data[row * self.cols + target] += addend;
        }
    }

    pub(crate) fn negate_row(&mut self, row: usize) {
        for col in 0..self.cols {
            let value = -self.at(row, col);
            self.data[row * self.cols + col] = value;
        }
    }

    pub(crate) fn negate_column(&mut self, col: usize) {
        for row in 0..self.rows {
            let value = -self.at(row, col);
            self.data[row * self.cols + col] = value;
        }
    }
}

impl core::fmt::Display for IntMatrix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("[")?;
        for row in 0..self.rows {
            if row > 0 {
                f.write_str("; ")?;
            }
            for col in 0..self.cols {
                if col > 0 {
                    f.write_str(" ")?;
                }
                core::fmt::Display::fmt(self.at(row, col), f)?;
            }
        }
        f.write_str("]")
    }
}

#[cfg(test)]
mod tests {
    use super::IntMatrix;
    use crate::algebra::Z;
    use crate::error::MatrixError;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn construction_checks_shape() {
        assert!(IntMatrix::new(2, 2, vec![Z::from(1); 4]).is_ok());
        assert_eq!(
            IntMatrix::new(2, 2, vec![Z::from(1); 3]).unwrap_err(),
            MatrixError::DataLength {
                expected: 4,
                found: 3
            }
        );
        assert_eq!(
            IntMatrix::from_rows([vec![1i64, 2], vec![3]]).unwrap_err(),
            MatrixError::RaggedRows {
                expected: 2,
                found: 1
            }
        );
    }

    #[test]
    fn identity_is_neutral() {
        let m = IntMatrix::from_rows([[1i64, 2, 3], [4, 5, 6]]).unwrap();
        assert_eq!(m.multiply(&IntMatrix::identity(3)).unwrap(), m);
        assert_eq!(IntMatrix::identity(2).multiply(&m).unwrap(), m);
    }

    #[test]
    fn multiplication_checks_dimensions() {
        let a = IntMatrix::from_rows([[1i64, 2, 3]]).unwrap();
        let b = IntMatrix::from_rows([[1i64], [2], [3]]).unwrap();
        assert_eq!(a.multiply(&b).unwrap().to_string(), "[14]");
        assert_eq!(
            b.multiply(&IntMatrix::identity(2)).unwrap_err(),
            MatrixError::DimensionMismatch { left: 1, right: 2 }
        );
    }

    #[test]
    fn apply_maps_vectors() {
        let val = IntMatrix::from_rows([[12i64, 19, 28]]).unwrap();
        let syntonic = [Z::from(-4), Z::from(4), Z::from(-1)];
        assert_eq!(val.apply(&syntonic).unwrap(), vec![Z::from(0)]);
        assert!(val.apply(&syntonic[..2]).is_err());
    }

    #[test]
    fn reverse_rows_is_an_involution() {
        let m = IntMatrix::from_rows([[1i64, 2], [3, 4], [5, 6]]).unwrap();
        assert_eq!(
            m.reverse_rows(),
            IntMatrix::from_rows([[5i64, 6], [3, 4], [1, 2]]).unwrap()
        );
        assert_eq!(m.reverse_rows().reverse_rows(), m);
    }

    #[test]
    fn transpose_is_an_involution() {
        let m = IntMatrix::from_rows([[1i64, -2, 3], [4, 5, -6]]).unwrap();
        assert_eq!(m.transpose().transpose(), m);
        assert_eq!((m.transpose().rows(), m.transpose().cols()), (3, 2));
    }

    #[test]
    fn column_selection() {
        let m = IntMatrix::from_rows([[1i64, 2, 3], [4, 5, 6]]).unwrap();
        assert_eq!(
            m.select_columns(&[2, 0]).unwrap(),
            IntMatrix::from_rows([[3i64, 1], [6, 4]]).unwrap()
        );
        assert!(m.select_columns(&[3]).is_err());
    }

    #[test]
    fn empty_shapes_are_representable() {
        let m = IntMatrix::zeros(0, 0);
        assert!(m.is_zero());
        assert_eq!(m.to_string(), "[]");
        let m = IntMatrix::zeros(3, 0);
        assert_eq!((m.rows(), m.cols()), (3, 0));
    }
}
