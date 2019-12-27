use super::Interrupt;
use crate::{cpu, segment};
use core::{marker::PhantomData, mem};

#[repr(C)]
#[repr(align(16))]
pub struct Idt {
    descriptors: [Descriptor; Self::NUM_VECTORS],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Descriptor {
    offset_low: u16,
    pub segment: segment::Selector,
    ist_offset: u16,
    pub attrs: Attrs,
    offset_mid: u16,
    offset_hi: u32,
    _zero: u32,
    // _f: PhantomData<T>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(transparent)]
pub struct Attrs(u8);

impl Descriptor {
    pub const fn null() -> Self {
        Self {
            offset_low: 0,
            segment: segment::Selector::null(),
            ist_offset: 0,
            attrs: Attrs::null(),
            offset_mid: 0,
            offset_hi: 0,
            _zero: 0,
            // _f: PhantomData,
        }
    }

    fn set_handler(&mut self, handler: *const ()) -> &mut Attrs {
        self.segment = segment::code_segment();
        let addr = handler as u64;
        self.offset_low = addr as u16;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_hi = (addr >> 32) as u32;
        self.attrs.set_present(true).set_32_bit(false)
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum GateKind {
    Interrupt = 0b0001_0110,
    Trap = 0b0000_0111,
    Task = 0b0000_0101,
}

impl Attrs {
    const IS_32_BIT: u8 = 0b1000;
    const KIND_BITS: u8 = GateKind::Interrupt as u8 | GateKind::Trap as u8 | GateKind::Task as u8;
    const PRESENT_BIT: u8 = 0b1000_0000;
    const RING_BITS: u8 = 0b0111_0000;
    const RING_SHIFT: u8 = Self::RING_BITS.trailing_zeros() as u8;

    pub const fn null() -> Self {
        Self(0)
    }

    pub fn gate_kind(&self) -> GateKind {
        match self.0 & Self::KIND_BITS {
            0b0110 => GateKind::Interrupt,
            0b0111 => GateKind::Trap,
            0b0101 => GateKind::Task,
            bits => unreachable!("unexpected bit pattern {:#08b}", bits),
        }
    }

    pub fn is_32_bit(&self) -> bool {
        self.0 & Self::IS_32_BIT != 0
    }

    pub fn is_present(&self) -> bool {
        self.0 & Self::PRESENT_BIT == Self::PRESENT_BIT
    }

    pub fn ring(&self) -> cpu::Ring {
        cpu::Ring::from_u8(self.0 & Self::RING_BITS >> Self::RING_SHIFT)
    }

    pub fn set_gate_kind(&mut self, kind: GateKind) -> &mut Self {
        self.0 &= Self::IS_32_BIT | kind as u8;
        self
    }

    pub fn set_32_bit(&mut self, is_32_bit: bool) -> &mut Self {
        if is_32_bit {
            self.0 |= Self::IS_32_BIT;
        } else {
            self.0 &= !Self::IS_32_BIT;
        }
        self
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        if present {
            self.0 |= Self::PRESENT_BIT;
        } else {
            self.0 &= !Self::PRESENT_BIT;
        }
        self
    }

    pub fn set_ring(&mut self, ring: cpu::Ring) -> &mut Self {
        let ring = (ring as u8) << Self::RING_SHIFT;
        self.0 &= !Self::RING_BITS | ring;
        self
    }
}

impl Idt {
    const NUM_VECTORS: usize = 256;

    pub const fn new() -> Self {
        Self {
            descriptors: [Descriptor::null(); Self::NUM_VECTORS],
        }
    }

    pub fn load(&'static self) {
        let ptr = crate::cpu::DtablePtr::new(self);
        unsafe { asm!("lidt ($0)" :: "r" (&ptr) : "memory") }
    }
}

impl hal_core::interrupt::Control for Idt {
    type Vector = u8;

    unsafe fn disable(&mut self) {
        asm!("cli" :::: "volatile");
    }

    unsafe fn enable(&mut self) {
        asm!("sti" :::: "volatile");
    }

    fn is_enabled(&self) -> bool {
        unimplemented!("eliza do this one!!!")
    }

    unsafe fn register_handler_raw<I>(
        &mut self,
        irq: &I,
        handler: *const (),
    ) -> Result<(), hal_core::interrupt::RegistrationError>
    where
        I: hal_core::interrupt::Interrupt<Ctrl = Self>,
    {
        self.descriptors[irq.vector() as usize]
            // .cast_mut::<I::Handler>()
            .set_handler(handler);
        // TODO(eliza): validate this you dipshit!
        Ok(())
    }
}