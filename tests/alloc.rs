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
    /// Bytes asked for, measured the same way. A count alone cannot
    /// see a copy getting bigger: growing a list by copying it
    /// allocates once per iteration either way, and it is the SIZE of
    /// that one allocation that goes quadratic.
    static BYTES: Cell<Option<usize>> = const { Cell::new(None) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        BYTES.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + layout.size()));
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

/// How many bytes `f` asks the allocator for, added up.
fn bytes(f: impl FnOnce()) -> usize {
    BYTES.with(|c| c.set(Some(0)));
    f();
    BYTES.with(|c| c.replace(None)).unwrap_or(0)
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

/// Growing a list with `xs += [x]` must not cost the length of the
/// list every time round. `+` used to copy the whole list, which is a
/// quadratic amount of copying; extending in place when nothing else
/// holds the list is linear. The allocation COUNT cannot tell those
/// apart -- both allocate about once per iteration -- so this weighs
/// the bytes. Doubling the iterations doubles a linear appetite and
/// quadruples a quadratic one; the budget is three times, which no
/// quadratic run can meet and no linear one can miss. Both spellings
/// are held to it: `xs += [x]` and the long form it is short for.
#[test]
fn growing_a_list_costs_the_appends_not_the_squares() {
    for append in ["xs += [i]", "xs = xs + [i]"] {
        let run = |n: usize| {
            let src = format!("let xs = []; let i = 0; while i < {n} {{ {append}; i += 1; }}");
            bytes(|| {
                ting::run_source("bench", &src, std::io::sink(), Vec::new()).expect("runs");
            })
        };
        let a = run(2000);
        let b = run(4000);
        assert!(
            b < a * 3,
            "`{append}`: {a} bytes for 2000 appends, {b} for 4000 — the list is being copied"
        );
    }
}

/// The same for a string built a piece at a time. `s += x` has been
/// linear since compound assignment arrived; the long form it is
/// short for was not, and nothing held either of them to it.
#[test]
fn growing_a_string_costs_the_pieces_not_the_squares() {
    for append in ["s += \"abcdefghij\"", "s = s + \"abcdefghij\""] {
        let run = |n: usize| {
            let src = format!("let s = \"\"; let i = 0; while i < {n} {{ {append}; i += 1; }}");
            bytes(|| {
                ting::run_source("bench", &src, std::io::sink(), Vec::new()).expect("runs");
            })
        };
        let a = run(2000);
        let b = run(4000);
        assert!(
            b < a * 3,
            "`{append}`: {a} bytes for 2000 appends, {b} for 4000 — the string is being copied"
        );
    }
}

/// Some function mentions the name, so a call on the right could
/// reassign it and the old value has to be read before the call
/// runs. Reading it is cheap — the text is shared — and once the call
/// has had its chance the binding lets go of what was read, unless it
/// no longer holds it. So the append extends the text rather than
/// copying it, and a right-hand side that DOES reassign the name
/// still behaves exactly as it did.
#[test]
fn a_call_on_the_right_appends_in_place_even_when_a_function_names_it() {
    for append in ["s += str(i)", "s = s + str(i)"] {
        let run = |n: usize| {
            let src = format!(
                "let s = \"\"; let peek = fn() {{ return len(s); }}; let i = 0; while i < {n} {{ {append}; i += 1; }} peek();"
            );
            bytes(|| {
                ting::run_source("bench", &src, std::io::sink(), Vec::new()).expect("runs");
            })
        };
        let a = run(2000);
        let b = run(4000);
        assert!(
            b < a * 3,
            "`{append}`: {a} bytes for 2000 appends, {b} for 4000 — the string is being copied"
        );
    }
}

/// A frame slot is a binding no closure mentions, so nothing a call
/// does can reach it and no read of the binding is needed at all:
/// this is the cheaper path, and the commonest loop there is — a
/// report built with `s += str(x)` inside a function — takes it.
#[test]
fn a_call_on_the_right_still_appends_in_place_in_a_frame() {
    for append in ["s += str(i)", "s = s + str(i)"] {
        let run = |n: usize| {
            let src = format!(
                "fn build(n) {{ let s = \"\"; let i = 0; while i < n {{ {append}; i += 1; }} return s; }} build({n});"
            );
            bytes(|| {
                ting::run_source("bench", &src, std::io::sink(), Vec::new()).expect("runs");
            })
        };
        let a = run(2000);
        let b = run(4000);
        assert!(
            b < a * 3,
            "`{append}`: {a} bytes for 2000 appends, {b} for 4000 — the string is being copied"
        );
    }
}

/// Reading a string one character at a time must not cost the string
/// each time. `s[i]` and `slice` used to decode the whole thing into
/// a `Vec<char>` per call, which is four bytes per character of the
/// WHOLE string for one character of answer; walking end to end then
/// asked the allocator for the square. Doubling the length is what
/// tells that apart from a walk that stops where it is asked to.
#[test]
fn reading_a_string_by_index_does_not_cost_the_string_each_time() {
    for read in ["s[j]", "slice(s, j, j + 1)"] {
        let run = |n: usize| {
            let src = format!(
                "let s = \"\"; let i = 0; while i < {n} {{ s += \"abcdefghij\"; i += 1; }} let j = 0; let c = 0; while j < len(s) {{ if {read} == \"e\" {{ c += 1; }} j += 1; }}"
            );
            bytes(|| {
                ting::run_source("bench", &src, std::io::sink(), Vec::new()).expect("runs");
            })
        };
        // 2000 characters read one at a time. Reading the name is a
        // pointer copy and the walk to the character allocates
        // nothing, so the only allocation per read is the
        // one-character answer: this measures 0.04 bytes per
        // character per read. It measured 2 when a string owned its
        // text and every read copied it, and 9 when each read also
        // decoded the whole string into a `Vec<char>`.
        let n = 2000;
        let per_read = run(n / 10) as f64 / (n * n) as f64;
        assert!(
            per_read < 0.5,
            "`{read}`: {per_read:.4} bytes per character per read over {n} characters — a read is copying or decoding the string, not just pointing at it"
        );
    }
}
