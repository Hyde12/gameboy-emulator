__Introduction__
This README is mostly just for me and my future self when I look back on this project and don’t know wtf I was doing or how it works. All entirely based on my understanding.

The Game Boy (gonna write it as GB/gb for now on and everythings mostly based on the emulator I wrote) specifications can be found in the PanDocs, which is the best and most annoying piece of documentation I’ve ever read. It has everything but lwk scattered everywhere it was a whole puzzle.

__What happens per tick (usually)__
The Gameboy has a 16-bit unsigned program counter register (which starts at $0100) and everytime the CPU fetches an instruction, it reads the memory bus at the address the program counter is in and increments it. The CPU then decodes this based on the CPU instructions set and the program goes on.

__Interrupts__
Every now and then a program instruciton will cause an interrupt. Genuinely don’t know what this is for, probably just to ensure that some stuff are handled immediately. The priority of the interrupts go from the rightmost bit to the leftmost. 

They are only handled when IME is set and the Interrupt Flag and Interrupt Enable registers corresponding to the bit are 1. 

Interrupt handling is checked at the start of the tick (before a fetch for the next instruction is issued) this then pushes the current program counter to the CPU stack and jumps to the instruction depending on the interrupt handled. (see pandocs)

__Timing__
Each instruction takes an X amount of T-cycles, again see the CPU instructions for this. This is just a count to keep every piece of hardware synced up. In my implementation, since reading and writing from the bus, for example, takes 4 T-cycles, I call a function and loop the T-cycle to increment each time and check the related registers that are affected. The dot count for the rendering is also checked in this loop.
