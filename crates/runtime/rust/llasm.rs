//! SR's CPU-state ABI. Keep offsets in sync with pinned llasm_cpu.h.
#[repr(C)]
#[derive(Default)]
pub struct Cpu {
    pub registers: [u32; 9],
    pub top: u32,
    pub status: u32,
    pub control: u32,
    pub floating: [f64; 8],
    pub base: u64,
    pub result: i64,
    pub stack_bottom: usize,
    pub stack_top: usize,
}

impl Cpu {
    unsafe fn operand(&self, kind: u32, value: u32, narrow: bool) -> u64 {
        match kind {
            0 => self.floating[value as usize].to_bits(),
            1 => u64::from(self.registers[value as usize]),
            2 => {
                let pointer = (self.base + u64::from(value)) as usize as *const u8;
                if narrow {
                    u64::from(unsafe { pointer.cast::<u32>().read_unaligned() })
                } else {
                    unsafe { pointer.cast::<u64>().read_unaligned() }
                }
            }
            3 => u64::from(value),
            _ => std::process::abort(),
        }
    }
}

/// # Safety
/// `cpu` points to SR's live CPU state; operands come from the translator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_rotate(
    cpu: *mut Cpu,
    register: u32,
    width: u32,
    shift: u32,
    right: u32,
    count: u32,
) {
    let cpu = unsafe { &mut *cpu };
    let count = count & 31;
    if count == 0 {
        return;
    }
    let mask = u32::MAX >> (32 - width);
    let value = (cpu.registers[register as usize] >> shift) & mask;
    let rotate = count % width;
    let result = if rotate == 0 {
        value
    } else if right != 0 {
        ((value >> rotate) | (value << (width - rotate))) & mask
    } else {
        ((value << rotate) | (value >> (width - rotate))) & mask
    };
    let carry = if right != 0 {
        result >> (width - 1)
    } else {
        result & 1
    };
    cpu.registers[8] = (cpu.registers[8] & !1) | carry;
    if count == 1 {
        let overflow = if right != 0 {
            (result >> (width - 1)) ^ (result >> (width - 2))
        } else {
            (result >> (width - 1)) ^ carry
        } & 1;
        cpu.registers[8] = (cpu.registers[8] & !0x800) | (overflow << 11);
    }
    cpu.registers[register as usize] =
        (cpu.registers[register as usize] & !(mask << shift)) | (result << shift);
}

/// # Safety
/// `cpu` is a live SR state and the register slice is valid for `width`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_shift(
    cpu: *mut Cpu,
    register: u32,
    width: u32,
    shift: u32,
    kind: u32,
    count: u32,
) {
    let cpu = unsafe { &mut *cpu };
    let count = count & 31;
    if count == 0 {
        return;
    }
    let mask = u32::MAX >> (32 - width);
    let value = (cpu.registers[register as usize] >> shift) & mask;
    let signed = (value << (32 - width)) as i32 >> (32 - width);
    let result = match kind {
        0 => value << count,
        1 => value >> count,
        2 => (signed >> count.min(width - 1)) as u32,
        _ => std::process::abort(),
    } & mask;
    let carry = if kind == 0 {
        if count <= width {
            (value >> (width - count)) & 1
        } else {
            0
        }
    } else if count <= width {
        (value >> (count - 1)) & 1
    } else if kind == 2 {
        value >> (width - 1)
    } else {
        0
    };
    let parity = u32::from((result as u8).count_ones().is_multiple_of(2)) << 2;
    let zero = u32::from(result == 0) << 6;
    let sign = (result >> (width - 1)) << 7;
    cpu.registers[8] = (cpu.registers[8] & !0xc5) | carry | parity | zero | sign;
    if count == 1 {
        let overflow = match kind {
            0 => (result >> (width - 1)) ^ carry,
            1 => value >> (width - 1),
            _ => 0,
        };
        cpu.registers[8] = (cpu.registers[8] & !0x800) | (overflow << 11);
    }
    cpu.registers[register as usize] =
        (cpu.registers[register as usize] & !(mask << shift)) | (result << shift);
}

/// # Safety
/// The guest EDI range contains ECX readable dwords in the direction of DF.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_scasd(cpu: *mut Cpu, repeat_not_equal: u32) -> u32 {
    let cpu = unsafe { &mut *cpu };
    let mut value = 0;
    while cpu.registers[1] != 0 {
        let pointer = (cpu.base + u64::from(cpu.registers[7])) as usize as *const u32;
        value = unsafe { pointer.read_unaligned() };
        cpu.registers[7] = if cpu.registers[8] & 0x400 != 0 {
            cpu.registers[7].wrapping_sub(4)
        } else {
            cpu.registers[7].wrapping_add(4)
        };
        cpu.registers[1] -= 1;
        if (value == cpu.registers[0]) == (repeat_not_equal != 0) {
            break;
        }
    }
    value
}

/// Guest CPU capabilities describe translated instructions, independently of the
/// host CPU. Advertise a Pentium MMX with x87 and MMX only, without a hardware
/// timestamp counter, SSE, OS services or host-specific instruction extensions.
///
/// # Safety
/// `cpu` points to SR's live CPU state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_cpuid(cpu: *mut Cpu) {
    let cpu = unsafe { &mut *cpu };
    let (eax, ebx, ecx, edx) = match cpu.registers[0] {
        0 => (
            1,
            u32::from_le_bytes(*b"Genu"),
            u32::from_le_bytes(*b"ntel"),
            u32::from_le_bytes(*b"ineI"),
        ),
        1 => (0x543, 0, 0, (1 << 23) | 1),
        _ => (0, 0, 0, 0),
    };
    cpu.registers[0] = eax;
    cpu.registers[3] = ebx;
    cpu.registers[1] = ecx;
    cpu.registers[2] = edx;
}

/// # Safety
/// `cpu` points to the live SR state; width is 16 or 32 as emitted by metadata.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_compare(cpu: *mut Cpu, left: u32, right: u32, width: u32) {
    let cpu = unsafe { &mut *cpu };
    let mask = u32::MAX >> (32 - width);
    let (left, right) = (left & mask, right & mask);
    let result = left.wrapping_sub(right) & mask;
    let flags = u32::from(left < right)
        | (u32::from((result as u8).count_ones().is_multiple_of(2)) << 2)
        | ((left ^ right ^ result) & 0x10)
        | (u32::from(result == 0) << 6)
        | ((result >> (width - 1)) << 7)
        | ((((left ^ right) & (left ^ result)) >> (width - 1)) << 11);
    cpu.registers[8] = (cpu.registers[8] & !0x8d5) | flags;
}

fn packed(left: u64, right: u64, width: u32, subtract: bool) -> u64 {
    let mask = (1u64 << width) - 1;
    let mut result = 0;
    for shift in (0..64).step_by(width as usize) {
        let a = (left >> shift) & mask;
        let b = (right >> shift) & mask;
        let value = if subtract {
            a.wrapping_sub(b)
        } else {
            a.wrapping_add(b)
        };
        result |= (value & mask) << shift;
    }
    result
}

fn packed_shift(value: u64, count: u64, width: u32, right: bool) -> u64 {
    if count >= u64::from(width) {
        return 0;
    }
    if width == 64 {
        return if right {
            value >> count
        } else {
            value << count
        };
    }
    let mask = (1u64 << width) - 1;
    let mut result = 0;
    for shift in (0..64).step_by(width as usize) {
        let lane = (value >> shift) & mask;
        let lane = if right { lane >> count } else { lane << count };
        result |= (lane & mask) << shift;
    }
    result
}

fn pack_signed_dwords(left: u64, right: u64) -> u64 {
    let lanes = [
        left as i32,
        (left >> 32) as i32,
        right as i32,
        (right >> 32) as i32,
    ];
    lanes
        .into_iter()
        .enumerate()
        .fold(0, |result, (index, lane)| {
            result | ((lane.clamp(i16::MIN as i32, i16::MAX as i32) as u16 as u64) << (index * 16))
        })
}

fn arithmetic_shift(value: u64, count: u64, width: u32) -> u64 {
    let count = count.min(u64::from(width - 1)) as u32;
    let mask = (1u64 << width) - 1;
    (0..64).step_by(width as usize).fold(0, |result, shift| {
        let lane = ((value >> shift) & mask) as i64;
        let signed = (lane << (64 - width)) >> (64 - width);
        result | (((signed >> count) as u64 & mask) << shift)
    })
}

fn signed_lane(value: u64, shift: u32, width: u32) -> i64 {
    ((value >> shift) << (64 - width)) as i64 >> (64 - width)
}

fn pack_bytes(left: u64, right: u64, unsigned: bool) -> u64 {
    let (min, max) = if unsigned { (0, 255) } else { (-128, 127) };
    [left, right]
        .into_iter()
        .enumerate()
        .fold(0, |result, (half, value)| {
            (0..4).fold(result, |result, lane| {
                let byte = signed_lane(value, lane * 16, 16).clamp(min, max) as u8;
                result | (u64::from(byte) << (half * 32 + lane as usize * 8))
            })
        })
}

fn unpack(left: u64, right: u64, width: u32, high: bool) -> u64 {
    let mask = (1u64 << width) - 1;
    let start = if high { 32 } else { 0 };
    (0..32 / width).fold(0, |result, lane| {
        let shift = start + lane * width;
        result
            | (((left >> shift) & mask) << (lane * width * 2))
            | (((right >> shift) & mask) << (lane * width * 2 + width))
    })
}

fn multiply_words(left: u64, right: u64, high: bool) -> u64 {
    (0..4).fold(0, |result, lane| {
        let product = signed_lane(left, lane * 16, 16) * signed_lane(right, lane * 16, 16);
        let value = if high { product >> 16 } else { product } as u16;
        result | (u64::from(value) << (lane * 16))
    })
}

fn multiply_add(left: u64, right: u64) -> u64 {
    (0..2).fold(0, |result, pair| {
        let sum: i64 = (0..2)
            .map(|lane| {
                let shift = (pair * 2 + lane) * 16;
                signed_lane(left, shift, 16) * signed_lane(right, shift, 16)
            })
            .sum();
        result | (u64::from(sum as u32) << (pair * 32))
    })
}

fn compare_lanes(left: u64, right: u64, width: u32, greater: bool) -> u64 {
    let mask = (1u64 << width) - 1;
    (0..64).step_by(width as usize).fold(0, |result, shift| {
        let a = signed_lane(left, shift, width);
        let b = signed_lane(right, shift, width);
        result
            | (if if greater { a > b } else { a == b } {
                mask
            } else {
                0
            } << shift)
    })
}

/// # Safety
/// `cpu` and any encoded memory operands must be valid for the instruction's
/// width. Register indices and operation IDs come from the pinned translator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_mmx(
    cpu: *mut Cpu,
    operation: u32,
    destination_kind: u32,
    destination: u32,
    source_kind: u32,
    source: u32,
) {
    let cpu = unsafe { &mut *cpu };
    let narrow = operation == 0;
    let right = unsafe { cpu.operand(source_kind, source, narrow) };
    let left = if operation < 2 {
        0
    } else {
        unsafe { cpu.operand(destination_kind, destination, false) }
    };
    let result = match operation {
        0 => right & 0xffff_ffff,
        1 => right,
        2 => left ^ right,
        3 => left & right,
        4 => !left & right,
        5 => left | right,
        6..=8 => packed(left, right, 8 << (operation - 6), false),
        9..=11 => packed(left, right, 8 << (operation - 9), true),
        12 => packed_shift(left, right, 64, false),
        13 => packed_shift(left, right, 64, true),
        14 => packed_shift(left, right, 16, false),
        15 => packed_shift(left, right, 16, true),
        16 => packed_shift(left, right, 32, false),
        17 => packed_shift(left, right, 32, true),
        18 => pack_signed_dwords(left, right),
        19 => arithmetic_shift(left, right, 16),
        20 => arithmetic_shift(left, right, 32),
        21 => pack_bytes(left, right, false),
        22 => pack_bytes(left, right, true),
        23..=25 => unpack(left, right, 8 << (operation - 23), false),
        26..=28 => unpack(left, right, 8 << (operation - 26), true),
        29 => multiply_words(left, right, false),
        30 => multiply_words(left, right, true),
        31 => multiply_add(left, right),
        32..=34 => compare_lanes(left, right, 8 << (operation - 32), false),
        35..=37 => compare_lanes(left, right, 8 << (operation - 35), true),
        _ => std::process::abort(),
    };
    match destination_kind {
        0 => cpu.floating[destination as usize] = f64::from_bits(result),
        1 => cpu.registers[destination as usize] = result as u32,
        2 => {
            let pointer = (cpu.base + u64::from(destination)) as usize as *mut u8;
            if narrow {
                unsafe { pointer.cast::<u32>().write_unaligned(result as u32) };
            } else {
                unsafe { pointer.cast::<u64>().write_unaligned(result) };
            }
        }
        _ => std::process::abort(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn comparison_flags_match_hardware() {
        let mut seed = 0x8735_92abu32;
        let edges = [
            0,
            1,
            0x7fff,
            0x8000,
            0xffff,
            0x7fff_ffff,
            0x8000_0000,
            u32::MAX,
        ];
        for iteration in 0..2048 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let left = if iteration < edges.len() {
                edges[iteration]
            } else {
                seed
            };
            for right in edges {
                for width in [16, 32] {
                    let flags: u64;
                    unsafe {
                        if width == 16 {
                            std::arch::asm!("cmp ax, dx", "pushfq", "pop {flags}",
                                in("eax") left, in("edx") right, flags = lateout(reg) flags);
                        } else {
                            std::arch::asm!("cmp eax, edx", "pushfq", "pop {flags}",
                                in("eax") left, in("edx") right, flags = lateout(reg) flags);
                        }
                    }
                    let mut cpu = Cpu::default();
                    cpu.registers[8] = 0x602;
                    unsafe { x86_compare(&mut cpu, left, right, width) };
                    assert_eq!(
                        cpu.registers[8] & 0x8d5,
                        flags as u32 & 0x8d5,
                        "left={left:x} right={right:x} width={width}"
                    );
                    assert_eq!(cpu.registers[8] & !0x8d5, 0x602);
                }
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn mmx_matches_hardware() {
        macro_rules! reference {
            ($instruction:literal, $left:expr, $right:expr) => {{
                let mut values = [$left, $right, 0u64];
                unsafe { std::arch::asm!(
                    "movq mm0, [{p}]", "movq mm1, [{p} + 8]",
                    $instruction, "movq [{p} + 16], mm0", "emms",
                    p = in(reg) values.as_mut_ptr(),
                    out("mm0") _, out("mm1") _, options(nostack),
                ) };
                values[2]
            }};
        }
        let mut seed = 0x91ab_35fd_aacc_1273u64;
        for iteration in 0..512 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let left = seed;
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let right = if iteration < 66 { iteration } else { seed };
            for operation in 0..38 {
                let expected = match operation {
                    0 => right & 0xffff_ffff,
                    1 => right,
                    2 => reference!("pxor mm0, mm1", left, right),
                    3 => reference!("pand mm0, mm1", left, right),
                    4 => reference!("pandn mm0, mm1", left, right),
                    5 => reference!("por mm0, mm1", left, right),
                    6 => reference!("paddb mm0, mm1", left, right),
                    7 => reference!("paddw mm0, mm1", left, right),
                    8 => reference!("paddd mm0, mm1", left, right),
                    9 => reference!("psubb mm0, mm1", left, right),
                    10 => reference!("psubw mm0, mm1", left, right),
                    11 => reference!("psubd mm0, mm1", left, right),
                    12 => reference!("psllq mm0, mm1", left, right),
                    13 => reference!("psrlq mm0, mm1", left, right),
                    14 => reference!("psllw mm0, mm1", left, right),
                    15 => reference!("psrlw mm0, mm1", left, right),
                    16 => reference!("pslld mm0, mm1", left, right),
                    17 => reference!("psrld mm0, mm1", left, right),
                    18 => reference!("packssdw mm0, mm1", left, right),
                    19 => reference!("psraw mm0, mm1", left, right),
                    20 => reference!("psrad mm0, mm1", left, right),
                    21 => reference!("packsswb mm0, mm1", left, right),
                    22 => reference!("packuswb mm0, mm1", left, right),
                    23 => reference!("punpcklbw mm0, mm1", left, right),
                    24 => reference!("punpcklwd mm0, mm1", left, right),
                    25 => reference!("punpckldq mm0, mm1", left, right),
                    26 => reference!("punpckhbw mm0, mm1", left, right),
                    27 => reference!("punpckhwd mm0, mm1", left, right),
                    28 => reference!("punpckhdq mm0, mm1", left, right),
                    29 => reference!("pmullw mm0, mm1", left, right),
                    30 => reference!("pmulhw mm0, mm1", left, right),
                    31 => reference!("pmaddwd mm0, mm1", left, right),
                    32 => reference!("pcmpeqb mm0, mm1", left, right),
                    33 => reference!("pcmpeqw mm0, mm1", left, right),
                    34 => reference!("pcmpeqd mm0, mm1", left, right),
                    35 => reference!("pcmpgtb mm0, mm1", left, right),
                    36 => reference!("pcmpgtw mm0, mm1", left, right),
                    37 => reference!("pcmpgtd mm0, mm1", left, right),
                    _ => unreachable!(),
                };
                let mut cpu = Cpu {
                    floating: [f64::from_bits(left); 8],
                    ..Default::default()
                };
                cpu.floating[1] = f64::from_bits(right);
                cpu.registers[8] = 0x8d7;
                unsafe { x86_mmx(&mut cpu, operation, 0, 0, 0, 1) };
                assert_eq!(
                    cpu.floating[0].to_bits(),
                    expected,
                    "op={operation} left={left:x} right={right:x}"
                );
                assert_eq!(cpu.registers[8], 0x8d7);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn byte_rotates_match_hardware() {
        for value in 0u32..256 {
            for count in 0u32..64 {
                for right in 0..2 {
                    let mut actual = value as u8;
                    let flags: u64;
                    unsafe {
                        if right != 0 {
                            std::arch::asm!("ror al, cl", "pushfq", "pop {flags}", inout("al") actual, in("cl") count as u8, flags = out(reg) flags);
                        } else {
                            std::arch::asm!("rol al, cl", "pushfq", "pop {flags}", inout("al") actual, in("cl") count as u8, flags = out(reg) flags);
                        }
                    }
                    let mut cpu = Cpu::default();
                    cpu.registers[0] = 0xabcd_1200 | value;
                    cpu.registers[8] = 0x8d6;
                    unsafe { x86_rotate(&mut cpu, 0, 8, 0, right, count) };
                    assert_eq!(cpu.registers[0], 0xabcd_1200 | u32::from(actual));
                    if count & 31 != 0 {
                        assert_eq!(cpu.registers[8] & 1, flags as u32 & 1);
                    }
                    if count & 31 == 1 {
                        assert_eq!(cpu.registers[8] & 0x800, flags as u32 & 0x800);
                    }
                    if count & 31 == 0 {
                        assert_eq!(cpu.registers[8], 0x8d6);
                    }
                    assert_eq!(cpu.registers[8] & !0x801, 0x8d6 & !0x801);
                }
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn variable_shifts_match_hardware() {
        macro_rules! reference {
            ($instruction:literal, $value:expr, $count:expr) => {{
                let mut value = $value;
                let flags: u64;
                unsafe { std::arch::asm!($instruction, "pushfq", "pop {flags}",
                    inout("eax") value, in("cl") $count as u8, flags = out(reg) flags); }
                (value, flags as u32)
            }};
        }
        let mut seed = 0xabcd1234u32;
        for _ in 0..512 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            for count in 0..64 {
                for kind in 0..3 {
                    let (expected, flags) = match kind {
                        0 => reference!("shl eax, cl", seed, count),
                        1 => reference!("shr eax, cl", seed, count),
                        _ => reference!("sar eax, cl", seed, count),
                    };
                    let mut cpu = Cpu::default();
                    cpu.registers[0] = seed;
                    cpu.registers[8] = 0x8d7;
                    unsafe { x86_shift(&mut cpu, 0, 32, 0, kind, count) };
                    assert_eq!(cpu.registers[0], expected);
                    if count & 31 == 0 {
                        assert_eq!(cpu.registers[8], 0x8d7);
                    } else {
                        assert_eq!(cpu.registers[8] & 0xc5, flags & 0xc5);
                    }
                    if count & 31 == 1 {
                        assert_eq!(cpu.registers[8] & 0x800, flags & 0x800);
                    }
                }
            }
        }
    }

    #[test]
    fn cpu_layout_matches_sr() {
        assert_eq!(std::mem::offset_of!(Cpu, floating), 48);
        assert_eq!(std::mem::offset_of!(Cpu, base), 112);
        assert_eq!(std::mem::offset_of!(Cpu, stack_bottom), 128);
        assert_eq!(std::mem::size_of::<Cpu>(), 144);
    }

    #[test]
    fn signed_pack_saturates_and_arithmetic_shifts_extend_sign() {
        assert_eq!(
            pack_signed_dwords(0x0000_8000_ffff_7fff, 0x8000_0000_7fff_ffff),
            0x8000_7fff_7fff_8000
        );
        assert_eq!(
            arithmetic_shift(0x8000_7fff_ffff_0001, 1, 16),
            0xc000_3fff_ffff_0000
        );
        assert_eq!(
            arithmetic_shift(0x8000_7fff_ffff_0001, u64::MAX, 16),
            0xffff_0000_ffff_0000
        );
        assert_eq!(
            arithmetic_shift(0x8000_0000_7fff_ffff, 32, 32),
            0xffff_ffff_0000_0000
        );
    }

    #[test]
    fn packed_lanes_wrap_independently_and_overshifts_zero() {
        assert_eq!(packed(u64::MAX, 0x0101_0101_0101_0101, 8, false), 0);
        assert_eq!(packed(0, 0x0001_0001_0001_0001, 16, true), u64::MAX);
        assert_eq!(packed_shift(u64::MAX, 16, 16, false), 0);
        assert_eq!(packed_shift(u64::MAX, 64, 64, true), 0);
        assert_eq!(packed_shift(u64::MAX, 1, 16, false), 0xfffe_fffe_fffe_fffe);
    }
}
