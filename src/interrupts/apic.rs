#![allow(dead_code)]

use io::{UnsafePort};


pub struct Apic {

}

impl Apic {

}

fn apic_enabled() -> bool {
    use raw_cpuid::CpuId;
    let cpu_id = CpuId::new();

    match cpu_id.get_feature_info() {
        Some(vf) => vf.has_apic(),
        None => false,
    }
}

pub unsafe fn init() {
    assert_has_not_been_called!("Must only initialize the APIC once!");
    disable_pic();
    if !apic_enabled() {
        panic!("The kernel required APIC to operate!");
    }
}

unsafe fn disable_pic() {
    const PIC1_DATA_PORT: u16 = 0x21;
    const PIC2_DATA_PORT:u16 = 0xa1;
    const PIC_DISABLE_COMMAND: u8 = 0xff;

    let mut pic1_port: UnsafePort<u8> = UnsafePort::new(PIC1_DATA_PORT);
    let mut pic2_port: UnsafePort<u8> = UnsafePort::new(PIC2_DATA_PORT);

    pic2_port.write(PIC_DISABLE_COMMAND);
    pic1_port.write(PIC_DISABLE_COMMAND);
}