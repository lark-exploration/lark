#![cfg(test)]

use crate::text::Text;
use map::FxIndexSet;

#[test]
fn extract() {
    let text1 = Text::from("big string");
    let text2 = text1.extract(0..3);
    assert_eq!(text1, "big string");
    assert_eq!(text2, "big");
    assert_ne!(text1, text2);
}

#[test]
fn in_hash() {
    let text1 = Text::from("big string");
    let text2 = text1.extract(0..3);
    let text3 = Text::from("big");

    assert_ne!(text1, text2);
    assert_ne!(text1, text3);
    assert_eq!(text2, text3);
    assert_eq!(text3, text2);

    let mut map = FxIndexSet::default();
    assert!(map.insert(text2));
    assert!(!map.insert(text3)); // already present
}
