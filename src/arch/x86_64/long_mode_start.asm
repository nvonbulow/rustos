global long_mode_start

extern rust_main

section .text
bits 64
long_mode_start:
  ; Load 0 into all data segment registers
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  call rust_main

  ; Print Okay to Screen
  mov rax, 0x2f592f412f4b2f4f
  mov qword [0xb8000], rax
  cli
  hlt
