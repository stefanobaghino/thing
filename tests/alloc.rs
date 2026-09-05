//! What the matcher allocates, which no other test can see.
//!
//! The gain of v2.113.0 is that a search does not allocate per
//! character: the thread lists, the `seen` vector and the epsilon
//! stack are reused across positions, and capture slots are shared
//! rather than copied. Every other test compares what the matcher
//! ANSWERS, and the answers were already right — so the wiring could
//! come out and only a benchmark would notice. This counts instead.
//!
//! The allocator is global, so this file is a test binary of its own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// None when not measuring, so other threads and the harness
    /// itself pay nothing. `const` init keeps the accessor from
    /// allocating, which would be recursive.
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many times `f` allocates.
fn allocations(f: impl FnOnce()) -> usize {
    COUNT.with(|c| c.set(Some(0)));
    f();
    COUNT.with(|c| c.replace(None)).unwrap_or(0)
}

/// A search over ten times the input must not allocate ten times as
/// often. The counts are not asserted to be equal — a longer subject
/// reaches more positions and a restart may still grow a thread list —
/// only that they do not scale with the input, which is what
/// allocating per position looks like.
#[test]
fn a_search_does_not_allocate_per_character() {
    for pattern in ["needle", "^(a|b)+x$", "co(de)=([0-9]+)", "[a-z]+[0-9]+"] {
        let re = ting::regex::Regex::new(pattern).expect("pattern compiles");
        let short: Vec<char> = "abcabcabc de code=12 zz".chars().collect();
        let long: Vec<char> = short
            .iter()
            .copied()
            .cycle()
            .take(short.len() * 10)
            .collect();

        let a = allocations(|| {
            re.find_at(&short, 0);
        });
        let b = allocations(|| {
            re.find_at(&long, 0);
        });

        // Ten times the characters, and the budget is a small constant
        // over the short run rather than a multiple of it.
        assert!(
            b <= a + 8,
            "{pattern}: {a} allocations over {} chars, {b} over {} — \
             allocation is scaling with the input",
            short.len(),
            long.len(),
        );
    }
}

/// The counter has to be real, or the test above passes on nothing.
#[test]
fn the_allocation_counter_sees_allocations() {
    let none = allocations(|| {});
    let some = allocations(|| {
        let v: Vec<u8> = Vec::with_capacity(64);
        std::hint::black_box(&v);
    });
    assert_eq!(none, 0, "an empty closure allocates nothing");
    assert!(some > 0, "a vector allocation is counted");
}
