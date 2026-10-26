//! The READMEs' examples are compiled as doctests from the crate docs, as rustdoc
//! would try to compile the READMEs' indented license URL too.

#[test]
fn readme_examples_are_the_doctested_ones() {
    let docs = include_str!("../src/lib.rs")
        .lines()
        .filter_map(|l| l.strip_prefix("//!"))
        .map(|l| l.strip_prefix(' ').unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n");
    for readme in [include_str!("../README.md"), include_str!("../README/README_CN.md")] {
        let examples: Vec<&str> =
            readme.split("```rust\n").skip(1).filter_map(|b| b.split("```").next()).collect();
        assert_eq!(examples.len(), 3);
        for code in examples {
            assert!(docs.contains(code), "not in src/lib.rs's docs:\n{code}");
        }
    }
}
