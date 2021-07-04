// https://austinmorlan.com/posts/chip8_emulator/
// rust implment
const FONTSET_START_ADDR: u16 = 0x50;
const START_ADDR: u16 = 0x200;
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct Chip8 {
    registers: [u8; 16], // general registers
    ir: u16,             // index register
    pc: u16,             // program counter
    sp: u8,              // stack pointer
    memory: [u8; 4096],  // 4k memory
    stack: [u16; 16],    // stack
    dt: u8,              // delay timer
    st: u8,              // sound timer
    keypad: [u8; 16],    // keypads
    video: [u8; 8 * 32], // display buffer
    opcode: u8,          // opcode
}

enum Opcode {
    X00E0,      // CLS -> clear the display
    X00EE,      // RET -> return from a subroutine
    X1nnn(u16), // JP addr -> jump to location nnn
    X2nnn(u16), // CALL addr -> call subroutine at nnn
    X3xkk(u16), // SE Vx, byte -> skip next instruction if Vx = kk
    X4xkk(u16), // SE Vx, byte -> skip next instruction if Vx != kk
    X5xy0(u16), // SE Vx, Vy -> skip next instruction if Vx = Vy
    X6xkk(u16), // LD Vx, byte -> Set Vx = kk
    X7xkk(u16), // ADD Vx, byte -> Set Vx = Vx + kk
    X8xy0(u16), // LD Vx, Vy -> Set Vx = Vy
    X8xy1(u16), // OR Vx, Vy -> Set Vx = Vx OR Vy
    X8xy2(u16), // AND Vx, Vy -> Set Vx = Vx AND Vy
    X8xy3(u16), // XOR Vx, Vy -> Set Vx = Vx XOR Vy
    X8xy4(u16), // ADD Vx, Vy -> Set Vx = Vx + Vy, set VF = carry (if Vx + Vy > 255 then 1 else 0)
    X8xy5(u16), // SUB Vx, Vy -> Set Vx = Vx - Vy, set VF = Not borrow (if Vx > Vy then 1 else 0)
    X8xy6(u16), // SHR Vx -> Set Vx = Vx SHR 1
    X8xy7(u16), // SUBN Vx, Vy -> Set Vx = Vy - Vx, set VF = Not borrow (if Vy > Vx then 1 else 0)
    X8xyE(u16), // SHL Vx {, Vy} -> Set Vx = Vx SHL 1
    X9xy0(u16), // SNE Vx, Vy -> skip next instruction if Vx != Vy
    XAnnn(u16), // LD I, addr -> Set I = nnn
    XBnnn(u16), // JP V0, addr -> Jump to location nnn + v0
    XCxkk(u16), // RND Vx, byte -> Set Vx = random byte AND kk
    XDxyn(u16), // DRW Vx, Vy, nibble -> Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision
    XEx9E(u16), // SKP Vx -> Skip next instruction if key with the value of Vx is pressed
    XExA1(u16), // SKNP Vx -> Skip next instruction if key with the value of Vx if not pressed
    XFx07(u16), // LD Vx, DT -> Set Vx = delay timer value
    XFx0A(u16), // LD Fx K -> Wait for a key press, store the value of the key in Vx
    XFx15(u16), // LD DT, Vx -> Set delay timer = Vx
    XFx18(u16), // LD ST, Vx -> Set sound timer = Vx
    XFx1E(u16), // ADD I, Vx -> Set I = I + Vx
    XFx29(u16), // LD F, Fx, -> Set I = location of sprite for digit Vx
    XFx33(u16), // LD B, Vx -> Store BCD representation of Vx in memory locations I, I+1, and I+2
    XFx55(u16), // LD [I], Vx -> Store registers V0 through Vx in memory starting at location I
    XFx65(u16), // LD Vx, [I] -> Read registers V0 through Vx from memory starting at location I
}

impl Chip8 {
    fn new() -> Self {
        let mut memory = [0x00; 4096];
        for i in 0..FONTSET.len() {
            memory[(FONTSET_START_ADDR as usize + i)] = FONTSET[i];
        }

        Chip8 {
            registers: [0x00; 16],
            ir: 0x00,
            pc: 0x00,
            sp: 0x00,
            memory,
            stack: [0x00; 16],
            dt: 0x00,
            st: 0x00,
            keypad: [0x00; 16],
            video: [0x00; 8 * 32],
            opcode: 0x00,
        }
    }
    fn cls(&mut self) {
        self.video.fill(0x00);
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn jp(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn call(&mut self, addr: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = addr;
    }

    fn operate(&mut self, opcode: Opcode) {
        match opcode {
            Opcode::X00E0 => self.cls(),
            Opcode::X00EE => self.ret(),
            Opcode::X1nnn(op) => self.jp(op & 0x0FFF),
            Opcode::X2nnn(op) => self.call(op & 0x0FFF),
            Opcode::X3xkk(_) => todo!(),
            Opcode::X4xkk(_) => todo!(),
            Opcode::X5xy0(_) => todo!(),
            Opcode::X6xkk(_) => todo!(),
            Opcode::X7xkk(_) => todo!(),
            Opcode::X8xy0(_) => todo!(),
            Opcode::X8xy1(_) => todo!(),
            Opcode::X8xy2(_) => todo!(),
            Opcode::X8xy3(_) => todo!(),
            Opcode::X8xy4(_) => todo!(),
            Opcode::X8xy5(_) => todo!(),
            Opcode::X8xy6(_) => todo!(),
            Opcode::X8xy7(_) => todo!(),
            Opcode::X8xyE(_) => todo!(),
            Opcode::X9xy0(_) => todo!(),
            Opcode::XAnnn(_) => todo!(),
            Opcode::XBnnn(_) => todo!(),
            Opcode::XCxkk(_) => todo!(),
            Opcode::XDxyn(_) => todo!(),
            Opcode::XEx9E(_) => todo!(),
            Opcode::XExA1(_) => todo!(),
            Opcode::XFx07(_) => todo!(),
            Opcode::XFx0A(_) => todo!(),
            Opcode::XFx15(_) => todo!(),
            Opcode::XFx18(_) => todo!(),
            Opcode::XFx1E(_) => todo!(),
            Opcode::XFx29(_) => todo!(),
            Opcode::XFx33(_) => todo!(),
            Opcode::XFx55(_) => todo!(),
            Opcode::XFx65(_) => todo!(),
        }
    }
}

fn main() {
    let mut chip8 = Chip8::new();
}
