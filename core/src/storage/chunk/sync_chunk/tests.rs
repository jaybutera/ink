// Copyright 2018-2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::{
    env,
    storage::{
        alloc::{
            AllocateUsing,
            BumpAlloc,
        },
        Flush,
        Key,
    },
    test_utils::run_test,
};

fn dummy_chunk() -> SyncChunk<u32> {
    unsafe {
        let mut alloc = BumpAlloc::from_raw_parts(Key([0x0; 32]));
        SyncChunk::allocate_using(&mut alloc)
    }
}

#[test]
fn simple() {
    run_test(|| {
        const TEST_LEN: u32 = 5;

        let mut chunk = dummy_chunk();

        // Invariants after initialization
        for i in 0..TEST_LEN {
            assert_eq!(chunk.get(i), None);
        }

        // Store some elements
        for i in 0..TEST_LEN {
            chunk.set(i, i);
            assert_eq!(chunk.get(i), Some(&i));
        }

        // Clear all elements.
        for i in 0..TEST_LEN {
            chunk.clear(i);
            assert_eq!(chunk.get(i), None);
        }
    })
}

#[test]
fn take_put() {
    run_test(|| {
        let mut chunk = dummy_chunk();

        // Take empty cell yields `None`
        assert_eq!(chunk.take(5), None);
        // Replace into the same yields `None` again
        assert_eq!(chunk.put(5, 42), None);
        // Taking now should yield the inserted value
        assert_eq!(chunk.take(5), Some(42));
    })
}

#[test]
fn replace() {
    run_test(|| {
        let mut chunk = dummy_chunk();

        // Replace some with none.
        assert_eq!(chunk.put(0, 42), None);
        // Again will yield previous result.
        assert_eq!(chunk.put(0, 42), Some(42));

        // After clearing it will be none again.
        chunk.clear(0);
        assert_eq!(chunk.put(0, 42), None);
    })
}

#[test]
fn take() {
    run_test(|| {
        let mut chunk = dummy_chunk();

        // Remove at none.
        assert_eq!(chunk.take(0), None);
        // Again will yield none again.
        assert_eq!(chunk.take(0), None);
        // Also get will return none.
        assert_eq!(chunk.get(0), None);

        // After inserting it will yield the inserted value.
        chunk.set(0, 1337);
        // Before take returns the inserted value.
        assert_eq!(chunk.get(0), Some(&1337));
        // Remove yields the taken value.
        assert_eq!(chunk.take(0), Some(1337));
        // After take returns none again.
        assert_eq!(chunk.get(0), None);
    })
}

#[test]
fn count_rw_get() {
    // How many times we read or write from or to cells.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Loading from all cells.
    for i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.get(i);
        }
        assert_eq!(env::test::total_reads(), i as u64 + 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), 0);
}

#[test]
fn count_rw_get_repeat() {
    // How many times we repeat to read from the same cell.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Loading from all cells.
    for _i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.get(0);
        }
        assert_eq!(env::test::total_reads(), 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 0);
}

#[test]
fn count_rw_set() {
    // How many times we read or write from or to cells.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for i in 0..N {
        chunk.set(i, 42);
    }
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), N as u64);
}

#[test]
fn count_rw_set_repeat() {
    // How many times we write to the same cell.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for _i in 0..N {
        chunk.set(0, 42);
    }
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 1);
}

#[test]
fn count_rw_put() {
    // How many times we read or write from or to cells.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.put(i, 42);
        }
        assert_eq!(env::test::total_reads(), i as u64 + 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), N as u64);
}

#[test]
fn count_rw_put_repeat() {
    // How many times we put into the same cell.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for _i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.put(0, 42);
        }
        assert_eq!(env::test::total_reads(), 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 1);
}

#[test]
fn count_rw_take() {
    // How many times we take from cells.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.take(i);
        }
        assert_eq!(env::test::total_reads(), i as u64 + 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), N as u64);
    assert_eq!(env::test::total_writes(), N as u64);
}

#[test]
fn count_rw_take_repeat() {
    // How many times we take from the same cell.
    const N: u32 = 5;

    let mut chunk = dummy_chunk();

    // Assert clean read writes.
    assert_eq!(env::test::total_reads(), 0);
    assert_eq!(env::test::total_writes(), 0);

    // Writing to all cells.
    for _i in 0..N {
        #[allow(unused_must_use)]
        {
            chunk.take(0);
        }
        assert_eq!(env::test::total_reads(), 1);
        assert_eq!(env::test::total_writes(), 0);
    }
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 0);

    // Flush and check reads and writes.
    chunk.flush();
    assert_eq!(env::test::total_reads(), 1);
    assert_eq!(env::test::total_writes(), 1);
}
