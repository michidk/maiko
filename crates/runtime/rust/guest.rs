//! Register layouts shared with the translated assembly and native ABI fixtures.

#[repr(C)]
#[derive(Default)]
pub struct GuestFrame {
    pub esp: u64,
    pub ebp: u64,
    pub edi: u64,
    pub esi: u64,
    pub edx: u64,
    pub ecx: u64,
    pub ebx: u64,
    pub eax: u64,
    pub flags: u64,
}

impl GuestFrame {
    pub fn error(&mut self, code: u32) {
        self.eax = code.into();
        self.flags |= 1;
    }

    pub fn success(&mut self, value: u32) {
        self.eax = value.into();
        self.flags &= !1;
    }
}

#[repr(C)]
#[derive(Default)]
pub struct GuestRegs {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub flags: u32,
}
