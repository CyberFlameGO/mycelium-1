use crate::cpu;
use hal_core::interrupt::{Handlers, RegistrationError};

pub(crate) struct Pic {
    command: cpu::Port,
    data: cpu::Port
}

impl Pic {
    pub(crate) const fn new(command: u16, data: u16) -> Self {
        Self {
            command: cpu::Port::at(command),
            data: cpu::Port::at(data),
        }
    }
}

pub struct CascadedPic {
    primary: Pic,
    secondary: Pic,
}

impl CascadedPic {
    pub(crate) const fn new() -> Self {
        Self {
            primary: Pic::new(0x20, 0x21),
            secondary: Pic::new(0xa0, 0xa1),
        }
    }
}

impl hal_core::interrupt::Control for CascadedPic {
    type Arch = crate::X64;

    fn register_handlers<H>(&mut self) -> Result<(), hal_core::interrupt::RegistrationError>
    where
        H: Handlers<Self::Arch>,
    {
        Err(RegistrationError::other("x86_64 handlers must be registered via the IDT, not to the PIC interrupt component"))
    }

    unsafe fn disable(&mut self) {
        self.primary.data.writeb(0xff);
        self.secondary.data.writeb(0xff);
    }

    unsafe fn enable(&mut self) {
        // TODO(ixi): confirm this?? it looks like "disable" is "write a 1 to set the line masked"
        //            so maybe it stands to reason that writing a 0 unmasks an interrupt?
        self.primary.data.writeb(0x00);
        self.secondary.data.writeb(0x00);
    }

    fn is_enabled(&self) -> bool {
        unimplemented!("ixi do this one!!!")
    }
}

impl CascadedPic {
    pub(crate) unsafe fn set_irq_addresses(&mut self, primary_start: u8, secondary_start: u8) {
        let primary_mask = self.primary.data.readb();
        let secondary_mask = self.secondary.data.readb();

        const EXTENDED_CONFIG: u8 = 0x01u8; // if present, there are four initialization control words
        const PIC_INIT: u8 = 0x10u8; // reinitialize the 8259 PIC

        self.primary.command.writeb(PIC_INIT | EXTENDED_CONFIG);
        self.secondary.command.writeb(PIC_INIT | EXTENDED_CONFIG);
        self.primary.data.writeb(primary_start);
        self.secondary.data.writeb(secondary_start);
        self.primary.data.writeb(4); // magic number: secondary pic is at IRQ2 (how does 4 say this ???)
        self.secondary.data.writeb(2); // magic number: secondary pic has cascade identity 2 (??)
        self.primary.data.writeb(1); // 8086/88 (MCS-80/85) mode
        self.secondary.data.writeb(1); // 8086/88 (MCS-80/85) mode

        self.primary.data.writeb(primary_mask);
        self.secondary.data.writeb(secondary_mask);
    }
}
