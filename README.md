An ambitious attempt to make a GBA emulator.

## Test code
GBA currently boots in Thumb mode and PPU mode 3.
I don't have a way to load a rom yet, but I can feed it individual Thumb instructions.
The following code writes a red pixel to the top left.

```
0x2006
0x0600
0x211F
0x8001
```
