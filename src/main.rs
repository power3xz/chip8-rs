// https://austinmorlan.com/posts/chip8_emulator/
// chip8 emulator rust로 구현하기
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

#[derive(Debug)]
struct Chip8 {
    registers: [u8; 16], // general registers
    ir: u16,             // index register (address register)
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

#[derive(Debug)]
enum Opcode {
    X00E0,         // CLS -> clear the display
    X00EE,         // RET -> return from a subroutine
    X1nnn(u16),    // JP addr -> jump to location nnn
    X2nnn(u16),    // CALL addr -> call subroutine at nnn
    X3xkk(u8, u8), // SE Vx, byte -> skip next instruction if Vx = kk
    X4xkk(u8, u8), // SE Vx, byte -> skip next instruction if Vx != kk
    X5xy0(u8, u8), // SE Vx, Vy -> skip next instruction if Vx = Vy
    X6xkk(u8, u8), // LD Vx, byte -> Set Vx = kk
    X7xkk(u8, u8), // ADD Vx, byte -> Set Vx = Vx + kk
    X8xy0(u8, u8), // LD Vx, Vy -> Set Vx = Vy
    X8xy1(u8, u8), // OR Vx, Vy -> Set Vx = Vx OR Vy
    X8xy2(u8, u8), // AND Vx, Vy -> Set Vx = Vx AND Vy
    X8xy3(u8, u8), // XOR Vx, Vy -> Set Vx = Vx XOR Vy
    X8xy4(u8, u8), // ADD Vx, Vy -> Set Vx = Vx + Vy, set VF = carry (if Vx + Vy > 255 then 1 else 0)
    X8xy5(u8, u8), // SUB Vx, Vy -> Set Vx = Vx - Vy, set VF = Not borrow (if Vx > Vy then 1 else 0)
    X8xy6(u8, u8), // SHR Vx -> Set Vx = Vx SHR 1
    X8xy7(u8, u8), // SUBN Vx, Vy -> Set Vx = Vy - Vx, set VF = Not borrow (if Vy > Vx then 1 else 0)
    X8xyE(u16),    // SHL Vx {, Vy} -> Set Vx = Vx SHL 1
    X9xy0(u16),    // SNE Vx, Vy -> skip next instruction if Vx != Vy
    XAnnn(u16),    // LD I, addr -> Set I = nnn
    XBnnn(u16),    // JP V0, addr -> Jump to location nnn + v0
    XCxkk(u16),    // RND Vx, byte -> Set Vx = random byte AND kk
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
    X0000,      // continue
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
            pc: START_ADDR,
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

    fn jump(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn call(&mut self, addr: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = addr;
    }

    fn set(&mut self, vx: u8, byte: u8) {
        self.registers[vx as usize] = byte;
    }

    fn setxy(&mut self, vx: u8, vy: u8) {
        println!("{}, {}", vx, vy);
        self.registers[vx as usize] = self.registers[vy as usize];
    }

    fn add(&mut self, vx: u8, byte: u8) {
        self.registers[vx as usize] += byte;
    }

    fn read_opcode(&self) -> Opcode {
        let op_byte1 = self.memory[self.pc as usize] as u16;
        let op_byte2 = self.memory[(self.pc + 1) as usize] as u16;
        let opcode = op_byte1 << 8 | op_byte2;
        let c = ((opcode & 0xF000) >> 12) as u8;
        let x = ((opcode & 0x0F00) >> 8) as u8;
        let y = ((opcode & 0x00F0) >> 4) as u8;
        let d = (opcode & 0x000F) as u8;
        match (c, x, y, d) {
            (0, 0, 0, 0) => Opcode::X0000,
            (0x6, _, _, _) => {
                Opcode::X6xkk(((opcode & 0x0F00) >> 8) as u8, (opcode & 0x00FF) as u8)
            }
            (0x7, _, _, _) => {
                Opcode::X7xkk(((opcode & 0x0F00) >> 8) as u8, (opcode & 0x00FF) as u8)
            }
            (0x8, _, _, 0x0) => Opcode::X8xy0(
                ((opcode & 0x0F00) >> 8) as u8,
                ((opcode & 0x00F0) >> 4) as u8,
            ),
            (_, _, _, _) => panic!("not implemented!"),
        }
    }

    fn run(&mut self) {
        loop {
            let opcode = self.read_opcode();
            self.pc += 2;
            self.operate(opcode);
            if self.pc >= 4096 {
                break;
            }
        }
    }

    fn operate(&mut self, opcode: Opcode) {
        match opcode {
            Opcode::X00E0 => self.cls(),
            Opcode::X00EE => self.ret(),
            Opcode::X1nnn(op) => self.jump(op & 0x0FFF),
            Opcode::X2nnn(op) => self.call(op & 0x0FFF),
            Opcode::X3xkk(_, _) => todo!(),
            Opcode::X4xkk(_, _) => todo!(),
            Opcode::X5xy0(_, _) => todo!(),
            Opcode::X6xkk(vx, byte) => self.set(vx, byte),
            Opcode::X7xkk(vx, byte) => self.add(vx, byte),
            Opcode::X8xy0(vx, vy) => self.setxy(vx, vy),
            Opcode::X8xy1(_, _) => todo!(),
            Opcode::X8xy2(_, _) => todo!(),
            Opcode::X8xy3(_, _) => todo!(),
            Opcode::X8xy4(_, _) => todo!(),
            Opcode::X8xy5(_, _) => todo!(),
            Opcode::X8xy6(_, _) => todo!(),
            Opcode::X8xy7(_, _) => todo!(),
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
            Opcode::X0000 => (),
        }
    }
}

fn main() {
    let mut chip8 = Chip8::new();
    chip8.memory[0x200] = 0x60;
    chip8.memory[0x201] = 0x10;
    chip8.run();
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn op_6010_v0_equals_16() {
        let mut chip8 = Chip8::new();
        chip8.memory[0x200] = 0x60;
        chip8.memory[0x201] = 0x10;
        chip8.run();
        assert_eq!(chip8.registers[0], 16);
    }

    #[test]
    fn op_6311_v3_equals_17() {
        let mut chip8 = Chip8::new();
        chip8.memory[0x200] = 0x63;
        chip8.memory[0x201] = 0x11;
        chip8.run();
        assert_eq!(chip8.registers[3], 17);
    }

    #[test]
    fn op_6310_7311_v3_equals_33() {
        let mut chip8 = Chip8::new();
        chip8.memory[0x200] = 0x63;
        chip8.memory[0x201] = 0x10;
        chip8.memory[0x202] = 0x73;
        chip8.memory[0x203] = 0x11;
        chip8.run();
        assert_eq!(chip8.registers[3], 33);
    }

    #[test]
    fn op_6310_6208_8320_v3_equals_8() {
        let mut chip8 = Chip8::new();
        // 6310
        chip8.memory[0x200] = 0x63;
        chip8.memory[0x201] = 0x10;
        // 6280
        chip8.memory[0x202] = 0x62;
        chip8.memory[0x203] = 0x08;
        // 8320
        chip8.memory[0x204] = 0x83;
        chip8.memory[0x205] = 0x20;

        chip8.run();
        assert_eq!(chip8.registers[3], 8);
    }
}
