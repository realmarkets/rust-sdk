use super::{MAX_UNTRUSTED_NESTING, NestingTooDeep, check_nesting};

fn nested(levels: usize) -> String {
    return format!("{}{}", "[".repeat(levels), "]".repeat(levels));
}

#[test]
fn nesting_up_to_the_limit_is_accepted_and_one_more_is_refused() {
    assert_eq!(check_nesting(nested(128).as_bytes(), 128), Ok(()));
    assert_eq!(
        check_nesting(nested(129).as_bytes(), 128),
        Err(NestingTooDeep { limit: 128 })
    );
}

// objects nest like arrays, and the two mix
#[test]
fn objects_and_arrays_count_together() {
    assert_eq!(check_nesting(br#"{"a":[{"b":[1]}]}"#, 4), Ok(()));
    assert_eq!(
        check_nesting(br#"{"a":[{"b":[1]}]}"#, 3),
        Err(NestingTooDeep { limit: 3 })
    );
}

// a bracket inside a string is text, an escaped quote does not end the string
#[test]
fn brackets_inside_strings_do_not_count() {
    let text = format!(r#"{{"note":"{}","q":"a\"[[["}}"#, "[".repeat(1_000));

    assert_eq!(check_nesting(text.as_bytes(), 1), Ok(()));
}

// the attack: an unknown field a parser skips, nested far past the limit
#[test]
fn a_deep_unknown_field_is_refused_at_the_ingress_limit() {
    let payload = format!(r#"{{"owner":"0x1","x":{}}}"#, nested(20_000));

    assert_eq!(
        check_nesting(payload.as_bytes(), MAX_UNTRUSTED_NESTING),
        Err(NestingTooDeep {
            limit: MAX_UNTRUSTED_NESTING
        })
    );
}
