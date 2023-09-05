use frontend::FieldGC;
use frontend::Wire;

pub fn xor_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = bit_xor(a[i], b[i]);
    }

    out
}

pub fn not_64<F: FieldGC>(a: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = a[i].not(cs);
    }

    out
}

pub fn and_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = a[i].and(b[i], cs);
    }

    out
}

pub fn rotate_left_64<F: FieldGC>(a: [Wire<F>; 64], n: usize) -> [Wire<F>; 64] {
    let mut out = Vec::with_capacity(64);
    for i in 0..64 {
        out.push(a[(i + n) % 64]);
    }

    out.try_into().unwrap()
}

pub fn bit_xor<F: FieldGC>(a: Wire<F>, b: Wire<F>) -> Wire<F> {
    let cs = a.cs();
    a + b - a * b * cs.alloc_const(F::from(2u32))
}
