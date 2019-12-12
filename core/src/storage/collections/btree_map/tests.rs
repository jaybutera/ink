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

use crate::{
    storage::{
        alloc::{
            AllocateUsing,
            BumpAlloc,
            Initialize,
        },
        btree_map::impls::Entry,
        collections::btree_map::impls::NodeHandle,
        BTreeMap,
        Key,
    },
    test_utils::run_test,
};
use itertools::Itertools;

fn empty_map() -> BTreeMap<i32, i32> {
    unsafe {
        let mut alloc = BumpAlloc::from_raw_parts(Key([0x0; 32]));
        BTreeMap::allocate_using(&mut alloc).initialize_into(())
    }
}

fn filled_map() -> BTreeMap<i32, i32> {
    let mut map = empty_map();
    map.insert(5, 50);
    map.insert(42, 420);
    map.insert(1337, 13370);
    map.insert(77, 770);
    assert_eq!(map.len(), 4);
    map
}

/// Returns all edges in the tree as one Vec.
fn all_edges(map: &BTreeMap<i32, i32>) -> Vec<u32> {
    let mut v = Vec::new();
    let mut i = 0;
    let mut cnt = 0;
    loop {
        if i == map.header().node_count {
            break
        }

        // We iterate over all storage entities of the tree and skip vacant entities.
        let handle = NodeHandle::new(cnt);
        if let Some(node) = map.get_node(&handle) {
            let mut edges = node.edges.to_vec().into_iter().filter_map(|x| x).collect();
            v.append(&mut edges);
            i += 1;
        }
        cnt += 1;
    }
    v
}

fn every_edge_exists_only_once(map: &BTreeMap<i32, i32>) -> bool {
    let all_edges = all_edges(map);
    let uniqued: Vec<u32> = all_edges.clone().into_iter().unique().collect();

    let only_unique_edges = all_edges.len() == uniqued.len();
    if only_unique_edges == false {
        uniqued.iter().for_each(|x| {
            if all_edges.iter().any(|a| *a == *x) {
                eprintln!("duplicate {:?}", x);
            }
        });
    }
    only_unique_edges
}

/// Conducts repeated insert and remove operations into the map by iterating
/// over `xs`. For each odd number a defined number of insert operations
/// are executed. For each even number it's asserted that the previously
/// inserted elements are in the map and they are removed subsequently.
fn insert_and_remove(xs: Vec<i32>) {
    let mut map = empty_map();
    let mut cnt_inserts = 0;
    let mut previous_even_x = None;
    let number_inserts = 3;

    xs.iter().for_each(|x| {
        let x = *x;
        if x % 2 == 0 {
            // on even numbers we insert
            for a in x..x + number_inserts {
                if let None = map.insert(a, a * 10) {
                    cnt_inserts += 1;
                }
                assert_eq!(map.len(), cnt_inserts);
            }
            previous_even_x = Some(x);
        } else if x % 2 == 1 && previous_even_x.is_some() {
            // if it's an odd number and we inserted in the previous run we assert
            // that the insert worked correctly and remove the elements again.
            let x = previous_even_x.unwrap();
            for a in x..x + number_inserts {
                assert_eq!(map.get(&a), Some(&(a * 10)));
                assert_eq!(map.remove(&a), Some(a * 10));
                assert_eq!(map.get(&a), None);
                cnt_inserts -= 1;
                assert_eq!(map.len(), cnt_inserts);
            }
            previous_even_x = None;
        }
        assert!(every_edge_exists_only_once(&map));
    });
}

#[test]
fn empty_map_works() {
    run_test(|| {
        let map = empty_map();

        // Initial invariant.
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    })
}

#[test]
fn putting_on_same_key_works() {
    run_test(|| {
        // given
        let mut map = empty_map();

        // when
        assert_eq!(map.insert(42, 420), None);
        assert_eq!(map.len(), 1);
        assert_eq!(map.insert(42, 520), Some(420));

        // then
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&42), Some(&520));
    })
}

#[test]
fn first_put_filled() {
    run_test(|| {
        // given
        let mut map = filled_map();
        assert_eq!(map.get(&5), Some(&50));
        assert_eq!(map.get(&42), Some(&420));
        assert_eq!(map.get(&1337), Some(&13370));
        assert_eq!(map.get(&77), Some(&770));
        assert_eq!(map.get(&4), None);
        assert_eq!(map.len(), 4);

        // when
        assert_eq!(map.insert(4, 40), None);

        // then
        assert_eq!(map.get(&4), Some(&40));
        assert_eq!(map.len(), 5);
    })
}

#[test]
fn entry_api_works() {
    run_test(|| {
        let mut map = filled_map();
        assert_eq!(map.entry(5).key(), &5);
        assert_eq!(map.entry(-1).key(), &-1);
        assert_eq!(map.entry(997).or_insert(9970), &9970);
    });
}

#[test]
fn entry_api_works_with_strings_and_multiple_calls() {
    run_test(|| {
        // given
        let mut map = unsafe {
            let mut alloc = BumpAlloc::from_raw_parts(Key([0x0; 32]));
            BTreeMap::allocate_using(&mut alloc).initialize_into(())
        };
        let k = String::from("poneyland");
        map.entry(k.clone()).or_insert(12);

        // when
        match map.entry(k.clone()) {
            Entry::Occupied(mut o) => {
                *o.get_mut() += 10;
                assert_eq!(*o.get(), 22);

                *o.get_mut() += 2;
            }
            _ => unreachable!(),
        };

        // then
        assert_eq!(map.get(&k).expect("must be there"), &24);
    });
}

#[test]
fn remove_works() {
    run_test(|| {
        // given
        let mut map = empty_map();

        // when
        assert_eq!(map.insert(4, 40), None);
        assert_eq!(map.get(&4), Some(&40));
        assert_eq!(map.len(), 1);

        // then
        assert_eq!(map.remove(&4), Some(40));
        assert_eq!(map.get(&4), None);
        assert_eq!(map.len(), 0);
    })
}

#[test]
fn multiple_inserts_for_same_key_work() {
    run_test(|| {
        // given
        let mut map = empty_map();
        assert_eq!(map.insert(0, 10), None);

        // when
        assert_eq!(map.insert(0, 20), Some(10));
        assert_eq!(map.get(&0), Some(&20));
        assert_eq!(map.len(), 1);

        // then
        assert_eq!(map.remove(&0), Some(20));
        assert_eq!(map.get(&0), None);
        assert_eq!(map.len(), 0);
    })
}

#[test]
fn putting_and_removing_many_items_works() {
    run_test(|| {
        // given
        let mut map = empty_map();
        let mut len = map.len();
        for i in 1..200 {
            assert_eq!(map.insert(i, i * 10), None);
            len += 1;
            assert_eq!(map.len(), len);
        }
        let max_node_count = map.header().node_count;

        // when
        for i in 1..200 {
            assert_eq!(map.get(&i), Some(&(i * 10)));
            assert_eq!(map.remove(&i), Some(i * 10));
            assert_eq!(map.get(&i), None);
            len -= 1;
            assert_eq!(map.len(), len);
        }

        // then
        assert_eq!(map.len(), 0);
        assert_eq!(map.header().node_count, 0);
        for i in 0..max_node_count {
            assert!(map.get_node(&NodeHandle::new(i)).is_none());
        }
    })
}

#[test]
fn simple_insert_and_removal() {
    run_test(|| {
        // given
        let xs = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, 10];
        let mut map = empty_map();
        let mut len = 0;
        xs.iter().for_each(|i| {
            if let Some(_) = map.insert(*i, i * 10) {
                unreachable!("no element must already exist there");
            }
            len += 1;
            assert_eq!(map.len(), len);
        });
        let max_node_count = map.header().node_count;

        xs.iter().for_each(|k| {
            let v = *k * 10;
            assert_eq!(map.get(k), Some(&v));
            assert_eq!(map.contains_key(k), true);
            assert_eq!(map.get_key_value(k), Some((k, &v)));
        });

        // when
        xs.iter().for_each(|i| {
            match map.remove(&i) {
                Some(v) => {
                    assert_eq!(v, i * 10);
                    len -= 1;
                }
                None => unreachable!(),
            };
            assert_eq!(map.len(), len);
        });

        // then
        assert_eq!(map.len(), 0);
        assert_eq!(map.header().node_count, 0);
        for i in 0..max_node_count {
            assert!(map.get_node(&NodeHandle::new(i)).is_none());
        }
    })
}

#[test]
fn alternating_inserts_and_remove_works() {
    run_test(|| {
        // given
        let mut map = empty_map();
        let mut len = map.len();
        let ops = vec![
            200, // insert
            100, // remove
            100, // insert
            200, // remove
        ];
        let mut max_node_count = 0;

        // when
        ops.iter().enumerate().for_each(|(p, n)| {
            if p % 2 == 0 {
                // if it's an even array index we insert `n` elements
                for i in 1..*n {
                    assert_eq!(map.insert(i, i * 10), None);
                    len += 1;
                    assert_eq!(map.len(), len);

                    let nodes = map.header().node_count;
                    if nodes > max_node_count {
                        max_node_count = nodes;
                    }
                }
            } else {
                // on odd indices we remove `n` elements
                for i in 1..*n {
                    assert_eq!(map.get(&i), Some(&(i * 10)));
                    assert_eq!(map.remove(&i), Some(i * 10));
                    assert_eq!(map.get(&i), None);
                    len -= 1;
                    assert_eq!(map.len(), len);
                }
            }
        });

        // then
        assert_eq!(map.len(), 0);
        assert_eq!(map.header().node_count, 0);
        for i in 0..max_node_count {
            assert!(map.get_node(&NodeHandle::new(i)).is_none());
        }
    })
}

#[test]
fn sorted_insert_and_removal() {
    run_test(|| {
        // given
        let mut map = empty_map();
        let mut len = map.len();

        let xs = vec![
            -95, -89, -86, -67, -54, -13, -6, -1, 4, 13, 15, 21, 31, 40, 65,
        ];
        let mut xs = xs.clone();
        xs.sort_by(|a, b| a.cmp(b));
        xs = xs.into_iter().unique().collect();
        let mut max_node_count = 0;

        // first insert in sorted order
        xs.iter().for_each(|i| {
            assert_eq!(map.insert(*i, i * 10), None);
            len += 1;
            max_node_count += map.header().node_count;
            assert_eq!(map.len(), len);
            assert!(every_edge_exists_only_once(&map));
        });

        // when
        // remove from the back
        xs.sort_by(|a, b| b.cmp(a));
        xs.iter().for_each(|i| {
            assert_eq!(map.remove(&i), Some(i * 10));
            len -= 1;
            assert_eq!(map.len(), len);
            assert!(every_edge_exists_only_once(&map));
        });

        // then
        assert_eq!(map.len(), 0);
        assert_eq!(map.header().node_count, 0);
        for i in 0..max_node_count {
            assert!(map.get_node(&NodeHandle::new(i)).is_none());
        }
    })
}

/// These are some cases which in the past have shown to generate complex trees
/// for which the removal/insert operations touch all kinds of corner cases.
#[test]
fn complex_trees_work() {
    run_test(|| {
        let xs = [
            -72, -68, 36, -30, 0, -38, -74, -60, 4, -2, 28, -34, 60, -42, -14, 32, -48,
            18, -6, 24, -10, 40, 62, -64, 48, -56, 14, 3,
        ];
        insert_and_remove(xs.to_vec());

        let xs = [
            2, -30, -26, 0, -34, -4, -38, -42, -8, -56, 66, 34, 16, 36, -62, -12, -20,
            38, 30, -50, -66, 6, 70, 62, -16, 12, -70, 42, 31,
        ];
        insert_and_remove(xs.to_vec());

        let xs = [-2, -66, -44, 34, -6, 62, 2, 6, -30, -70, 30, -62, 7, -44, 7];
        insert_and_remove(xs.to_vec());
    })
}

#[quickcheck]
fn randomized_inserts_and_removes(xs: Vec<i32>) {
    run_test(|| {
        insert_and_remove(xs);
    })
}

/// Insert all items from `xs` and afterwards remove them again.
#[quickcheck]
fn randomized_insert_and_remove(xs: Vec<i32>) {
    run_test(|| {
        // given
        let mut map = empty_map();
        let mut len = map.len();
        xs.iter().for_each(|i| {
            if let None = map.insert(*i, i * 10) {
                len += 1;
            }
            assert_eq!(map.len(), len);
        });
        let max_node_count = map.header().node_count;
        xs.iter().for_each(|i| {
            assert_eq!(map.get(i), Some(&(*i * 10)));
        });

        // when
        xs.iter().for_each(|i| {
            if let Some(v) = map.remove(&i) {
                assert_eq!(v, i * 10);
                len -= 1;
            };
            assert_eq!(map.len(), len);
        });

        // then
        assert_eq!(map.len(), 0);
        assert_eq!(map.header().node_count, 0);
        for i in 0..max_node_count {
            assert!(map.get_node(&NodeHandle::new(i)).is_none());
        }
    })
}

#[quickcheck]
fn randomized_removes(xs: Vec<i32>, xth: usize) {
    run_test(|| {
        let mut map = empty_map();
        let mut len = map.len();

        let xth = {
            match xth {
                0 => 1,
                _ => xth,
            }
        };

        let xs: Vec<i32> = xs.into_iter().unique().collect();

        // first insert all
        xs.iter().for_each(|i| {
            assert_eq!(map.insert(*i, i * 10), None);
            len += 1;
            assert_eq!(map.len(), len);
        });

        // then remove every x'th
        xs.iter().enumerate().for_each(|(x, i)| {
            if x % xth == 0 {
                assert_eq!(map.remove(&i), Some(i * 10));
                len -= 1;
            }
            assert_eq!(map.len(), len);
        });

        // then everything else must still be get-able
        xs.iter().enumerate().for_each(|(x, i)| {
            if x % xth != 0 {
                assert_eq!(map.get(&i), Some(&(i * 10)));
            }
        });
    })
}