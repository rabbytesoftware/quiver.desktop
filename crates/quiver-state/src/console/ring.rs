/// Appends `add` to `list`, keeping only the newest `cap` items. Never mutates its inputs.
pub fn append_capped<T: Clone>(
    list: &[T],
    add: &[T],
    cap: usize,
) -> Vec<T> {
    let mut merged = Vec::with_capacity((list.len() + add.len()).min(cap.saturating_add(1)));
    merged.extend_from_slice(list);
    merged.extend_from_slice(add);
    if merged.len() > cap {
        merged.drain(..merged.len() - cap);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_under_the_cap_and_drops_the_oldest_over_it() {
        assert_eq!(append_capped(&[1, 2], &[3], 5), [1, 2, 3]);
        assert_eq!(append_capped(&[1, 2, 3], &[4, 5, 6], 4), [3, 4, 5, 6]);
    }

    #[test]
    fn nothing_to_add_still_trims_a_list_already_over_the_cap() {
        assert_eq!(append_capped(&[1, 2, 3, 4], &[], 2), [3, 4]);
        assert_eq!(append_capped::<i32>(&[], &[], 2), Vec::<i32>::new());
    }

    #[test]
    fn a_batch_larger_than_the_cap_keeps_its_newest() {
        assert_eq!(append_capped(&[1], &[2, 3, 4, 5], 2), [4, 5]);
    }

    #[test]
    fn inputs_are_never_mutated() {
        let list = vec![1, 2, 3];
        let add = vec![4];
        let _ = append_capped(&list, &add, 2);
        assert_eq!((list, add), (vec![1, 2, 3], vec![4]));
    }
}
