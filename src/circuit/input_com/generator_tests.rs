use crate::circuit::{Generator, Pos};

#[test]
fn value_at_non_loop_holds_last_value() {
    let generator = Generator::new(Pos::new(0, 0), vec![true, false], false);

    assert!(generator.value_at(0));
    assert!(!generator.value_at(1));
    assert!(!generator.value_at(2));
    assert!(!generator.value_at(10));
}

#[test]
fn value_at_loop_repeats_pattern() {
    let generator = Generator::new(Pos::new(0, 0), vec![true, false], true);

    assert!(generator.value_at(0));
    assert!(!generator.value_at(1));
    assert!(generator.value_at(2));
    assert!(!generator.value_at(3));
}

#[test]
fn value_at_loop_is_correct_across_u32_max_boundary() {
    // wasm32 では usize が 32 ビットのため tick as usize でビット切り捨てが起きる。
    // u64 演算を使うことで正しい値が返ることを確認する。
    //
    // パターン長 3 のループ: tick % 3 の期待値
    //   tick = u32::MAX     (= 4294967295): 4294967295 % 3 = 0 → true
    //   tick = u32::MAX + 1 (= 4294967296): 4294967296 % 3 = 1 → false
    //   tick = u32::MAX + 2 (= 4294967297): 4294967297 % 3 = 2 → true (pattern[2])
    let generator = Generator::new(Pos::new(0, 0), vec![true, false, true], true);

    assert!(generator.value_at(u64::from(u32::MAX)));
    assert!(!generator.value_at(u64::from(u32::MAX) + 1));
    assert!(generator.value_at(u64::from(u32::MAX) + 2));
}

#[test]
fn value_at_non_loop_saturates_at_last_beyond_u32_max() {
    // 非ループで tick が pattern 長を超えたら末尾の値を返し続ける。
    // u64 の大きな値でもインデックスが末尾に収まることを確認する。
    let generator = Generator::new(Pos::new(0, 0), vec![true, false], false);

    assert!(!generator.value_at(u64::from(u32::MAX)));
    assert!(!generator.value_at(u64::from(u32::MAX) + 1));
}
