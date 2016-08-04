Ok, back to the drawing board...

The whole communication between chips through callbacks or the Bus trait 
doesn't work that well, since the callback chains may cycle back to the original
caller (the snake bites its own tail...), also: RefCells :/

Here's a new idea: instead of the slightly messy callbacks where everything
can call into anything else at any time, per-chip work happens strictly sequential
within one 'opcode frame', for instance for a a system with a CPU, 1 PIO
and 1 CTC:

1. first the CPU does its thing
2. then PIO channel A does its thing
3. then PIO channel B does its thing
4. then CTC channel 0 ...
5.      CTC channel 1 ...
6.                  2 ...
7.                  3 ...
4. finally the CPU processes interrupt requests from step 2 and 3

The order in which those processing steps happen should follow 
the interrupt controller daisychain priorities in the system.

The main difference to before is that the CPU, PIO and CTC never talk
to each other directly, or indirectly through function calls.

Instead the new Bus structure is just a passive, shared, data storage
(really just a simple struct with public members)
where chips write to and read from. Things like 'write to PIO-A control',
'read from PIO-A data', 'request an interrupt' etc etc etc... are no longer
Bus trait functions, but simple data slots.

If the CPU executes an OUT instruction to write some value to a PIO or CTC
register it no longer calls a function on the Bus trait, which then 
calls into a PIO function, but instead it simply writes a value into 
a Bus member variable, and it's done.

Then when it's the PIO's turn it checks if a new value has been placed in 
one of it's input slots in the Bus, handles that, and optionally
places new values into its "output slots".

Chips basically communicate now by writing values, or flipping bits
in the Bus struct. A bit like deferred event handling, but without the 
overhead of creating and dispatching event objects.

                  Bus (just a simple data store now)
    +-----+      +---+                  
    | CPU |<---->|   |    +-------+     
    | OP  |      |   |<-->| PIO-A |
    +-----+      |   |    +-------+
                 |   |<-->| PIO-B |
                 |   |    +-------+
                 |   |<-->| CTC-0 |
                 |   |    +-------+
                 |   |<-->| CTC-1 |
                 |   |    +-------+
                 |   |<-->| CTC-2 |
                    .........
                 |   |
    +-----+      |   |
    | CPU |<---->|   |
    | IRQ?|      |   |
    +-----+      \   /
                  \ /

So Bus is now simply a struct with a number of simple data items. Each chip
gets a mut& to Bus, but only during it's 'dowork()' function, only one chip
needs access at any given time.

There *may* still be something like a System trait, because the CPU alone
doesn't know whether an OUT to a specific port should end up in the PIO-A
control register, or anywhere else. Thus there must be some user-provided
code which sits between the CPU and the Bus, and this code will
be different for each emulated system. But this user-code will never
forward function calls into other chips, it will only write values
into specific places in the Bus (and other user-provided code
will read those values back into other chips at later stages
in the processing chain).

Sounds like this could work, and efficiently :)

