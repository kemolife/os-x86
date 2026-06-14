pub unsafe fn strlen(s: *const u8) -> usize {
    let mut i = 0;
    while *s.add(i) != 0 {
        i += 1;
    }
    i
}

pub unsafe fn reverse(s: *mut u8) {
    let len = strlen(s);
    if len == 0 {
        return;
    }
    let (mut i, mut j) = (0usize, len - 1);
    while i < j {
        let c = *s.add(i);
        *s.add(i) = *s.add(j);
        *s.add(j) = c;
        i += 1;
        j -= 1;
    }
}

pub unsafe fn int_to_ascii(mut n: i32, str: *mut u8) {
    let sign = n;
    if n < 0 {
        n = -n;
    }
    let mut i: usize = 0;
    loop {
        *str.add(i) = (n % 10) as u8 + b'0';
        i += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if sign < 0 {
        *str.add(i) = b'-';
        i += 1;
    }
    *str.add(i) = 0;
    reverse(str);
}

pub unsafe fn hex_to_ascii(n: i32, str: *mut u8) {
    append(str, b'0');
    append(str, b'x');
    let mut zeros = false;

    let mut i = 28i32;
    while i > 0 {
        let tmp = ((n >> i) & 0xF) as u8;
        if tmp == 0 && !zeros {
            i -= 4;
            continue;
        }
        zeros = true;
        append(str, if tmp >= 0xA { tmp - 0xA + b'a' } else { tmp + b'0' });
        i -= 4;
    }
    let tmp = (n & 0xF) as u8;
    append(str, if tmp >= 0xA { tmp - 0xA + b'a' } else { tmp + b'0' });
}

pub unsafe fn append(s: *mut u8, c: u8) {
    let len = strlen(s);
    *s.add(len) = c;
    *s.add(len + 1) = 0;
}

pub unsafe fn backspace(s: *mut u8) {
    let len = strlen(s);
    if len > 0 {
        *s.add(len - 1) = 0;
    }
}

pub unsafe fn strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i = 0;
    while *s1.add(i) == *s2.add(i) {
        if *s1.add(i) == 0 {
            return 0;
        }
        i += 1;
    }
    *s1.add(i) as i32 - *s2.add(i) as i32
}
