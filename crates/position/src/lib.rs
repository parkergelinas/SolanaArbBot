//! Backward-compatible re-export of [`portfolio::position_sizing`].
//!
//! All existing consumers of `position` continue to work without modification.
//! New code should import from `portfolio` directly.

#![forbid(unsafe_code)]

pub use portfolio::compute_position_size;

#[cfg(test)]
mod tests {
    use super::compute_position_size;

    #[test]
    fn base_size_computed_via_re_export() {
        let sz = compute_position_size(250.0, 0.02, 0, 0, 0.05, 1.5);
        assert_eq!(sz, 5.0);
    }

    #[test]
    fn loss_reductions_via_re_export() {
        let sz = compute_position_size(250.0, 0.02, 0, 3, 0.05, 1.5);
        assert_eq!(sz, 2.56);
    }

    #[test]
    fn win_increases_cap_via_re_export() {
        let sz = compute_position_size(250.0, 0.02, 5, 0, 0.05, 1.5);
        assert!(sz <= 12.5);
    }
}
