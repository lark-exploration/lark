#![cfg(test)]

use crate::Seq;

#[test]
fn ne() {
    let x: Seq<u32> = (0..10).collect();
    let y: Seq<u32> = (0..20).collect();
    assert!(x != y);
}

#[test]
fn eq_same() {
    let x: Seq<u32> = (0..10).collect();
    let y: Seq<u32> = (0..10).collect();
    assert!(x == y);
}

#[test]
fn eq_select() {
    let mut x: Seq<u32> = (0..20).collect();
    x.select(10..20);

    let y: Seq<u32> = (10..20).collect();

    assert!(x == y);
}

#[test]
fn extend_empty() {
    let mut x: Seq<u32> = Seq::default();
    x.extend(0..5);
    assert_eq!(&[0, 1, 2, 3, 4], &x[..]);
}

#[test]
fn extend_non_empty() {
    let mut x: Seq<u32> = Seq::default();
    x.extend(0..5);
    x.extend(6..10);
    assert_eq!(&[0, 1, 2, 3, 4, 6, 7, 8, 9], &x[..]);
}

#[test]
fn extend_select() {
    let mut x: Seq<u32> = Seq::default();

    x.extend(0..5);
    x.select(3..5);
    assert_eq!(&[3, 4], &x[..]);

    x.extend(6..10);
    assert_eq!(&[3, 4, 6, 7, 8, 9], &x[..]);
}

#[test]
fn extend_extracted() {
    let mut x: Seq<u32> = Seq::default();
    x.extend(0..5);

    let mut y = x.extract(3..5);
    y.extend(6..10);

    assert_eq!(&[0, 1, 2, 3, 4], &x[..]);
    assert_eq!(&[3, 4, 6, 7, 8, 9], &y[..]);
}

#[test]
fn extend_cloned() {
    let mut x: Seq<u32> = Seq::default();
    x.extend(0..5);

    let mut y = x.clone();
    y.extend(6..10);

    assert_eq!(&[0, 1, 2, 3, 4], &x[..]);
    assert_eq!(&[0, 1, 2, 3, 4, 6, 7, 8, 9], &y[..]);
}
