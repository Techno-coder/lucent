# Lucent

A transparent systems language for linking and development on freestanding and embedded environments.

```
@@binary "flat"
@load 1024 * 1024
@architecture "x32"
module Loader
    module Header
        static MAGIC: u32 = 0xe85250d6
        static ARCHITECTURE: u32 = 0
        static HEADER_LENGTH: u32 = 
            Intrinsic.size(Header) as u32
        static CHECK: u32 = 0x100000000 - 
            (0xe85250d6 + HEADER_LENGTH)

        module EndTag
            static TYPE: u16 = 0
            static FLAGS: u16 = 0
            static SIZE: u32 = 8
            
    module Main
        root fn start()
            $esp = Intrinsic.end(STACK)
            check_multiboot()

        fn check_multiboot()
            if $eax != 0x36d76289:
                no_multiboot(0)

        fn no_multiboot(code: u8) never
            *(0xb8000 as *u32) = 0x4f524f45 
            *(0xb8004 as *u32) = 0x4f3a4f52 
            *(0xb8008 as *u32) = 0x4f204f20 
            *(0xb800a as *u32) = code
            inline x86.halt()       

    static STACK: [u8; 64 * 1024]
```

## Planned features
* Minimal code optimizations
* Fast and incremental compilation
* Arbitrary compile time evaluation
* Embed bytes directly into functions
* Ergonomic import of symbols from binary files

## Design
The Lucent language's core philosophy is **transparency**. Transparency (in this context) means that it should be easy to understand how the source code transforms into the final binary output. To that extent, all optimizations (or lack thereof) are a part of the language's [specification](specification.md).
