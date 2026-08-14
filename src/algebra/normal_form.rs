//! Smith and Hermite normal forms over the integers.
//!
//! Both are computed with exact arbitrary-precision arithmetic and both carry
//! their transformation matrices, so every claim they make is independently
//! checkable: `left * matrix * right == diagonal` for Smith, and
//! `matrix * transform == hermite` for Hermite (prompt section 10).
//!
//! Hermite normal form here is *canonical*: the same lattice always produces
//! the same basis, which is what makes lattice equality, deterministic
//! serialization, and reproducible generation possible (prompt section 53).
//! Smith normal form is not canonical in its transformation matrices, only in
//! its invariant factors, so nothing downstream depends on the particular `U`
//! and `V` produced here.

use alloc::vec::Vec;

use num_integer::Integer;
use num_traits::{Signed, Zero};

use crate::algebra::Z;
use crate::algebra::matrix::IntMatrix;

/// The Smith normal form of an integer matrix.
///
/// For an input `A` of shape `r` by `k`, this holds unimodular `U` (`r` by
/// `r`) and `V` (`k` by `k`) with
///
/// ```text
/// U A V = D
/// ```
///
/// where `D` is diagonal with positive entries `d_1 | d_2 | ... | d_rank`
/// followed by zeros. The inverses of `U` and `V` are retained because the
/// image and saturation constructions need them, and computing them by
/// accumulation is exact where a general inversion would not be.
///
/// UMT layer: L1/L2, exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithNormalForm {
    invariant_factors: Vec<Z>,
    rows: usize,
    cols: usize,
    left: IntMatrix,
    left_inverse: IntMatrix,
    right: IntMatrix,
    right_inverse: IntMatrix,
}

struct Workspace {
    d: IntMatrix,
    u: IntMatrix,
    u_inv: IntMatrix,
    v: IntMatrix,
    v_inv: IntMatrix,
}

impl Workspace {
    fn swap_rows(&mut self, first: usize, second: usize) {
        self.d.swap_rows(first, second);
        self.u.swap_rows(first, second);
        self.u_inv.swap_columns(first, second);
    }

    fn swap_columns(&mut self, first: usize, second: usize) {
        self.d.swap_columns(first, second);
        self.v.swap_columns(first, second);
        self.v_inv.swap_rows(first, second);
    }

    /// `row[target] += factor * row[source]`.
    fn add_scaled_row(&mut self, target: usize, source: usize, factor: &Z) {
        self.d.add_scaled_row(target, source, factor);
        self.u.add_scaled_row(target, source, factor);
        let negated = -factor;
        self.u_inv.add_scaled_column(source, target, &negated);
    }

    /// `column[target] += factor * column[source]`.
    fn add_scaled_column(&mut self, target: usize, source: usize, factor: &Z) {
        self.d.add_scaled_column(target, source, factor);
        self.v.add_scaled_column(target, source, factor);
        let negated = -factor;
        self.v_inv.add_scaled_row(source, target, &negated);
    }

    fn negate_row(&mut self, row: usize) {
        self.d.negate_row(row);
        self.u.negate_row(row);
        self.u_inv.negate_column(row);
    }
}

/// Finds the nonzero entry of smallest magnitude in the trailing submatrix.
fn find_pivot(matrix: &IntMatrix, from: usize) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, Z)> = None;
    for row in from..matrix.rows() {
        for col in from..matrix.cols() {
            let value = matrix.at(row, col);
            if value.is_zero() {
                continue;
            }
            let magnitude = value.abs();
            match &best {
                Some((_, _, current)) if *current <= magnitude => {}
                _ => best = Some((row, col, magnitude)),
            }
        }
    }
    best.map(|(row, col, _)| (row, col))
}

impl SmithNormalForm {
    /// Computes the Smith normal form of `matrix`.
    ///
    /// Terminates because every step either clears an entry or strictly
    /// decreases the magnitude of the current pivot, which is a positive
    /// integer.
    #[must_use]
    pub fn of(matrix: &IntMatrix) -> Self {
        let rows = matrix.rows();
        let cols = matrix.cols();
        let mut work = Workspace {
            d: matrix.clone(),
            u: IntMatrix::identity(rows),
            u_inv: IntMatrix::identity(rows),
            v: IntMatrix::identity(cols),
            v_inv: IntMatrix::identity(cols),
        };

        let mut invariant_factors = Vec::new();
        let limit = rows.min(cols);
        let mut step = 0;

        while step < limit {
            let Some((pivot_row, pivot_col)) = find_pivot(&work.d, step) else {
                break;
            };
            work.swap_rows(step, pivot_row);
            work.swap_columns(step, pivot_col);

            loop {
                let mut restart = false;

                // Clear the column below the pivot.
                for row in (step + 1)..rows {
                    if work.d.at(row, step).is_zero() {
                        continue;
                    }
                    let quotient = work.d.at(row, step) / work.d.at(step, step);
                    let negated = -quotient;
                    work.add_scaled_row(row, step, &negated);
                    if !work.d.at(row, step).is_zero() {
                        work.swap_rows(step, row);
                        restart = true;
                    }
                }
                if restart {
                    continue;
                }

                // Clear the row to the right of the pivot.
                for col in (step + 1)..cols {
                    if work.d.at(step, col).is_zero() {
                        continue;
                    }
                    let quotient = work.d.at(step, col) / work.d.at(step, step);
                    let negated = -quotient;
                    work.add_scaled_column(col, step, &negated);
                    if !work.d.at(step, col).is_zero() {
                        work.swap_columns(step, col);
                        restart = true;
                    }
                }
                if restart {
                    continue;
                }

                // Enforce the divisibility chain: the pivot must divide every
                // remaining entry, or the chain d_1 | d_2 | ... fails.
                let mut offending = None;
                'search: for row in (step + 1)..rows {
                    for col in (step + 1)..cols {
                        if !(work.d.at(row, col) % work.d.at(step, step)).is_zero() {
                            offending = Some(row);
                            break 'search;
                        }
                    }
                }
                match offending {
                    Some(row) => work.add_scaled_row(step, row, &Z::from(1)),
                    None => break,
                }
            }

            if work.d.at(step, step).is_negative() {
                work.negate_row(step);
            }
            invariant_factors.push(work.d.at(step, step).clone());
            step += 1;
        }

        Self {
            invariant_factors,
            rows,
            cols,
            left: work.u,
            left_inverse: work.u_inv,
            right: work.v,
            right_inverse: work.v_inv,
        }
    }

    /// The positive invariant factors `d_1 | d_2 | ... | d_rank`.
    #[must_use]
    pub fn invariant_factors(&self) -> &[Z] {
        &self.invariant_factors
    }

    /// The rank of the original matrix.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.invariant_factors.len()
    }

    /// The unimodular left factor `U`.
    #[must_use]
    pub fn left(&self) -> &IntMatrix {
        &self.left
    }

    /// The inverse of the left factor.
    #[must_use]
    pub fn left_inverse(&self) -> &IntMatrix {
        &self.left_inverse
    }

    /// The unimodular right factor `V`.
    #[must_use]
    pub fn right(&self) -> &IntMatrix {
        &self.right
    }

    /// The inverse of the right factor.
    #[must_use]
    pub fn right_inverse(&self) -> &IntMatrix {
        &self.right_inverse
    }

    /// The diagonal matrix `D`, rebuilt at the original shape.
    #[must_use]
    pub fn diagonal(&self) -> IntMatrix {
        let mut out = IntMatrix::zeros(self.rows, self.cols);
        for (index, factor) in self.invariant_factors.iter().enumerate() {
            out.set(index, index, factor.clone())
                .expect("invariant: rank does not exceed either dimension");
        }
        out
    }

    /// A basis for the kernel of the original matrix, as columns.
    ///
    /// These are the columns of `V` beyond the rank: `A V e_j = U^{-1} D e_j`,
    /// which vanishes exactly when `j >= rank`. The result is not canonical;
    /// callers that need determinism put it through
    /// [`HermiteNormalForm::column_of`].
    #[must_use]
    pub fn kernel_basis(&self) -> IntMatrix {
        let columns: Vec<usize> = (self.rank()..self.cols).collect();
        self.right
            .select_columns(&columns)
            .expect("invariant: kernel column indices are in range")
    }

    /// A basis for the saturation of the column lattice of the original
    /// matrix, as columns.
    ///
    /// The image is `U^{-1} span{d_i e_i}`, so its saturation - the set of
    /// ambient points some nonzero multiple of which lies in the image - is
    /// `U^{-1} span{e_i}` over the same index range.
    #[must_use]
    pub fn saturation_basis(&self) -> IntMatrix {
        let columns: Vec<usize> = (0..self.rank()).collect();
        self.left_inverse
            .select_columns(&columns)
            .expect("invariant: saturation column indices are in range")
    }

    /// Whether the column lattice of the original matrix is saturated in the
    /// ambient lattice.
    ///
    /// Equivalent to the quotient by that lattice being torsion-free, which
    /// holds exactly when every invariant factor is 1 (UMT-3.2 section 1.5).
    #[must_use]
    pub fn has_saturated_column_lattice(&self) -> bool {
        self.invariant_factors
            .iter()
            .all(|factor| *factor == Z::from(1))
    }
}

/// The canonical column-style Hermite normal form of an integer matrix.
///
/// For an input `A` this holds a unimodular `T` with `A T = H`, where the
/// nonzero columns of `H` are the canonical basis of the lattice generated by
/// the columns of `A`: pivots are positive, strictly descending in row index
/// from left to right, and every entry to the left of a pivot is reduced into
/// `[0, pivot)`.
///
/// Because the form is canonical, two matrices generate the same lattice
/// exactly when their Hermite bases are equal.
///
/// UMT layer: L1/L2, exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HermiteNormalForm {
    matrix: IntMatrix,
    transform: IntMatrix,
    pivots: Vec<(usize, usize)>,
}

impl HermiteNormalForm {
    /// Computes the canonical column-style Hermite normal form.
    #[must_use]
    pub fn column_of(matrix: &IntMatrix) -> Self {
        let rows = matrix.rows();
        let cols = matrix.cols();
        let mut h = matrix.clone();
        let mut transform = IntMatrix::identity(cols);
        let mut pivots: Vec<(usize, usize)> = Vec::new();
        let mut pivot_col = 0usize;

        let swap = |h: &mut IntMatrix, t: &mut IntMatrix, a: usize, b: usize| {
            h.swap_columns(a, b);
            t.swap_columns(a, b);
        };

        for row in 0..rows {
            if pivot_col >= cols {
                break;
            }
            let Some(found) = (pivot_col..cols).find(|col| !h.at(row, *col).is_zero()) else {
                continue;
            };
            swap(&mut h, &mut transform, pivot_col, found);

            for col in (pivot_col + 1)..cols {
                while !h.at(row, col).is_zero() {
                    let quotient = h.at(row, col) / h.at(row, pivot_col);
                    let negated = -quotient;
                    h.add_scaled_column(col, pivot_col, &negated);
                    transform.add_scaled_column(col, pivot_col, &negated);
                    if !h.at(row, col).is_zero() {
                        swap(&mut h, &mut transform, pivot_col, col);
                    }
                }
            }

            if h.at(row, pivot_col).is_negative() {
                h.negate_column(pivot_col);
                transform.negate_column(pivot_col);
            }

            pivots.push((row, pivot_col));
            pivot_col += 1;
        }

        // Canonicalize: reduce the entries left of each pivot into
        // `[0, pivot)`. Columns to the right already vanish in a pivot row,
        // and a column's own pivot lies below every earlier pivot row, so this
        // does not disturb the echelon structure.
        for (row, col) in pivots.clone() {
            let pivot = h.at(row, col).clone();
            for earlier in 0..col {
                let quotient = h.at(row, earlier).div_floor(&pivot);
                let negated = -quotient;
                h.add_scaled_column(earlier, col, &negated);
                transform.add_scaled_column(earlier, col, &negated);
            }
        }

        Self {
            matrix: h,
            transform,
            pivots,
        }
    }

    /// The full normal form `H`, at the shape of the input.
    #[must_use]
    pub fn matrix(&self) -> &IntMatrix {
        &self.matrix
    }

    /// The unimodular column transform `T` with `A T = H`.
    #[must_use]
    pub fn transform(&self) -> &IntMatrix {
        &self.transform
    }

    /// The pivot positions, as `(row, column)` pairs in column order.
    #[must_use]
    pub fn pivots(&self) -> &[(usize, usize)] {
        &self.pivots
    }

    /// The rank, that is, the number of nonzero columns.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.pivots.len()
    }

    /// The canonical basis of the generated lattice, as columns.
    #[must_use]
    pub fn basis(&self) -> IntMatrix {
        let columns: Vec<usize> = (0..self.rank()).collect();
        self.matrix
            .select_columns(&columns)
            .expect("invariant: pivot columns are in range")
    }
}

#[cfg(test)]
mod tests {
    use super::{HermiteNormalForm, SmithNormalForm};
    use crate::algebra::Z;
    use crate::algebra::matrix::IntMatrix;
    use alloc::vec;
    use alloc::vec::Vec;
    use num_traits::Zero;

    fn factors(form: &SmithNormalForm) -> Vec<i64> {
        form.invariant_factors()
            .iter()
            .map(|f| f.to_string().parse().unwrap())
            .collect()
    }

    /// `U A V = D`, and both transforms are unimodular, checked by exhibiting
    /// their inverses.
    fn check_smith(matrix: &IntMatrix) -> SmithNormalForm {
        let form = SmithNormalForm::of(matrix);

        let reconstructed = form
            .left()
            .multiply(matrix)
            .unwrap()
            .multiply(form.right())
            .unwrap();
        assert_eq!(reconstructed, form.diagonal(), "U A V = D");

        assert_eq!(
            form.left().multiply(form.left_inverse()).unwrap(),
            IntMatrix::identity(matrix.rows()),
            "U is unimodular"
        );
        assert_eq!(
            form.left_inverse().multiply(form.left()).unwrap(),
            IntMatrix::identity(matrix.rows())
        );
        assert_eq!(
            form.right().multiply(form.right_inverse()).unwrap(),
            IntMatrix::identity(matrix.cols()),
            "V is unimodular"
        );
        assert_eq!(
            form.right_inverse().multiply(form.right()).unwrap(),
            IntMatrix::identity(matrix.cols())
        );

        // Divisibility chain and positivity.
        for window in form.invariant_factors().windows(2) {
            assert!(!window[0].is_zero());
            assert!(
                (&window[1] % &window[0]).is_zero(),
                "d_i must divide d_(i+1): {window:?}"
            );
        }
        for factor in form.invariant_factors() {
            assert!(*factor > Z::zero(), "invariant factors are positive");
        }

        // The kernel basis really is in the kernel.
        let kernel = form.kernel_basis();
        let product = matrix.multiply(&kernel).unwrap();
        assert!(product.is_zero(), "A K = 0");
        assert_eq!(kernel.cols(), matrix.cols() - form.rank(), "kernel rank");

        form
    }

    #[test]
    fn twelve_edo_five_limit() {
        let val = IntMatrix::from_rows([[12i64, 19, 28]]).unwrap();
        let form = check_smith(&val);
        assert_eq!(form.rank(), 1);
        assert_eq!(factors(&form), vec![1]);
        assert!(form.has_saturated_column_lattice(), "12-EDO is surjective");
    }

    #[test]
    fn six_edo_image_has_invariant_factor_two() {
        let val = IntMatrix::from_rows([[6i64, 10, 14]]).unwrap();
        let form = check_smith(&val);
        assert_eq!(form.rank(), 1);
        assert_eq!(factors(&form), vec![2], "image is 2Z");
        assert!(!form.has_saturated_column_lattice());
    }

    #[test]
    fn doubling_map_on_rank_one() {
        // F1: V = [2] : Z -> Z has trivial kernel but non-surjective image.
        let val = IntMatrix::from_rows([[2i64]]).unwrap();
        let form = check_smith(&val);
        assert_eq!(factors(&form), vec![2]);
        assert_eq!(form.kernel_basis().cols(), 0, "kernel is trivial");
    }

    #[test]
    fn zero_matrix_has_rank_zero() {
        let form = check_smith(&IntMatrix::zeros(3, 4));
        assert_eq!(form.rank(), 0);
        assert!(form.invariant_factors().is_empty());
        assert_eq!(form.kernel_basis().cols(), 4, "everything is in the kernel");
        assert_eq!(form.saturation_basis().cols(), 0);
    }

    #[test]
    fn empty_and_degenerate_shapes() {
        check_smith(&IntMatrix::zeros(0, 0));
        check_smith(&IntMatrix::zeros(0, 3));
        check_smith(&IntMatrix::zeros(3, 0));
    }

    #[test]
    fn square_with_negative_entries() {
        let matrix = IntMatrix::from_rows([[2i64, -4, 4], [-6, 6, 12], [10, -4, -16]]).unwrap();
        let form = check_smith(&matrix);
        assert_eq!(form.rank(), 3);
        // The product of the invariant factors is the determinant up to sign.
        let product: i64 = factors(&form).iter().product();
        assert_eq!(product, 336);
    }

    #[test]
    fn wide_and_tall_rectangles() {
        let wide = IntMatrix::from_rows([[2i64, 4, 6, 8], [3, 6, 9, 12]]).unwrap();
        let form = check_smith(&wide);
        assert_eq!(form.rank(), 1);
        assert_eq!(factors(&form), vec![1]);

        let tall = wide.transpose();
        let form = check_smith(&tall);
        assert_eq!(form.rank(), 1);
        assert_eq!(factors(&form), vec![1]);
    }

    #[test]
    fn large_integer_entries() {
        let big = Z::from(10).pow(30);
        // `big * [[1, 2], [3, 5]]`, whose integer part is unimodular.
        let matrix = IntMatrix::new(2, 2, vec![big.clone(), &big * 2, &big * 3, &big * 5]).unwrap();
        let form = check_smith(&matrix);
        assert_eq!(form.rank(), 2);
        assert_eq!(form.invariant_factors(), &[big.clone(), big]);
    }

    #[test]
    fn unsaturated_subgroup_is_detected() {
        // Twice the syntonic comma generates an unsaturated subgroup.
        let generators = IntMatrix::from_rows([[-8i64], [8], [-2]]).unwrap();
        let form = check_smith(&generators);
        assert_eq!(factors(&form), vec![2]);
        assert!(!form.has_saturated_column_lattice());

        // Its saturation is generated by the comma itself, up to sign.
        let saturation = HermiteNormalForm::column_of(&form.saturation_basis()).basis();
        let comma =
            HermiteNormalForm::column_of(&IntMatrix::from_rows([[-4i64], [4], [-1]]).unwrap())
                .basis();
        assert_eq!(saturation, comma);
    }

    /// `A T = H`, `T` unimodular, and the form is canonical.
    fn check_hermite(matrix: &IntMatrix) -> HermiteNormalForm {
        let form = HermiteNormalForm::column_of(matrix);
        assert_eq!(
            matrix.multiply(form.transform()).unwrap(),
            *form.matrix(),
            "A T = H"
        );

        // Echelon structure and reduced entries.
        let mut previous_row: Option<usize> = None;
        for (index, (row, col)) in form.pivots().iter().enumerate() {
            assert_eq!(index, *col, "pivot columns are the leading ones");
            if let Some(previous) = previous_row {
                assert!(*row > previous, "pivot rows strictly increase");
            }
            previous_row = Some(*row);
            let pivot = form.matrix().at(*row, *col);
            assert!(*pivot > Z::zero(), "pivots are positive");
            for earlier in 0..*col {
                let entry = form.matrix().at(*row, earlier);
                assert!(
                    *entry >= Z::zero() && entry < pivot,
                    "entries left of a pivot are reduced"
                );
            }
            for later in (*col + 1)..form.matrix().cols() {
                assert!(
                    form.matrix().at(*row, later).is_zero(),
                    "entries right of a pivot vanish in its row"
                );
            }
        }
        form
    }

    #[test]
    fn hermite_is_canonical_for_the_same_lattice() {
        // Two different generating sets of the same rank-2 lattice.
        let a = IntMatrix::from_rows([[2i64, 0], [0, 3]]).unwrap();
        let b = IntMatrix::from_rows([[2i64, 2], [3, 6]]).unwrap();
        let form_a = check_hermite(&a);
        let form_b = check_hermite(&b);
        assert_eq!(form_a.basis(), form_b.basis());
    }

    #[test]
    fn hermite_of_a_full_lattice_is_the_identity() {
        let generators = IntMatrix::from_rows([[1i64, 3, 5], [0, 1, 7]]).unwrap();
        let form = check_hermite(&generators);
        assert_eq!(form.basis(), IntMatrix::identity(2));
    }

    #[test]
    fn hermite_handles_zero_and_negative_input() {
        let form = check_hermite(&IntMatrix::zeros(3, 2));
        assert_eq!(form.rank(), 0);
        assert_eq!(form.basis().cols(), 0);

        let form = check_hermite(&IntMatrix::from_rows([[-4i64], [-6]]).unwrap());
        assert_eq!(form.basis(), IntMatrix::from_rows([[4i64], [6]]).unwrap());
    }

    #[test]
    fn hermite_of_a_rank_deficient_matrix() {
        let matrix = IntMatrix::from_rows([[1i64, 2, 3], [2, 4, 6], [3, 6, 9]]).unwrap();
        let form = check_hermite(&matrix);
        assert_eq!(form.rank(), 1);
        assert_eq!(
            form.basis(),
            IntMatrix::from_rows([[1i64], [2], [3]]).unwrap()
        );
    }
}
