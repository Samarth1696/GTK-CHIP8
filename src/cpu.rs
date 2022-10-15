use crate::bus::Bus;
use std::fmt;

pub const PROGRAM_START: u16 = 0x200;

pub struct Cpu {
    vx: [u8; 16],
    pc: u16,
    i: u16,
    ret_stack: Vec<u16>
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            vx: [0; 16],
            pc: PROGRAM_START,
            i: 0,
            ret_stack: Vec::<u16>::new()
        }
    }

    pub fn run_instruction(&mut self, bus: &mut Bus) {
        let hi = bus.ram_read_byte(self.pc) as u16;
        let lo = bus.ram_read_byte(self.pc + 1) as u16;
        let instruction: u16 = (hi << 8) | lo;
        println!("instruction: {:#X}:{:#X}, hi: {:#X}, lo: {:#X}", self.pc, instruction, lo, hi);
        
        let nnn = instruction & 0x0FFF;
        let nn = (instruction & 0x0FF) as u8;
        let n = (instruction & 0x00F) as u8;
        let x = ((instruction & 0x0F00) >> 8) as u8;
        let y = ((instruction & 0x00F0) >> 4) as u8;
        println!("nnn={:?}, nn={:?}, n={:?}, x={}, y={}", nnn, nn, n, x, y);

        match (instruction & 0xF000) >> 12 {
            0x0 => {
                match nn {
                    0xE0 => {
                        bus.clear_screen();
                        self.pc += 2;
                    }
                    0xEE => {
                        //return from subroutine
                        let addr = self.ret_stack.pop().unwrap();
                        self.pc = addr;
                    }
                    _ => panic!("Unrecognized 0x00** instruction {:#X}:{:#X}", self.pc, instruction),
                }
            }
            0x1 => {
                // goto nnn
                self.pc = nnn;
            }
            0x2 => {
                //Call subroutine at address NNN
                self.ret_stack.push(self.pc + 2);
                self.pc = nnn;
            }
            0x3 => {
                // if(Vx==nn)
                let vx = self.read_reg_vx(x);
                if vx == nn{
                    self.pc += 4;
                }else{
                    self.pc += 2;
                }
            },
            0x4 => {
                // if(Vx!=nn)
                let vx = self.read_reg_vx(x);
                if vx != nn{
                    self.pc += 4;
                }else{
                    self.pc += 2;
                }
            }
            0x6 => {
                // vx = nn
                self.write_reg_vx(x, nn);
                self.pc += 2;
            }
            0x7 => {
                let vx = self.read_reg_vx(x);
                self.write_reg_vx(x, vx.wrapping_add(nn));
                self.pc += 2;
            }
            0x8 => {
                let vy = self.read_reg_vx(y);
                    let vx = self.read_reg_vx(x);
                    
                match n {
                    0 => {
                        // Vx=Vy
                        self.write_reg_vx(x, vy);
                        self.pc += 2;
                    }
                    2 => {
                        // Vx &= Vy
                        self.write_reg_vx(x, vx & vy);
                        self.pc += 2;
                    }
                    3 => {
                        // Vx ^ Vy
                        self.write_reg_vx(x, vx ^ vy);
                    }
                    4 => {
                        // Vx += Vy
                        let sum: u16 = vx as u16 + vy as u16;
                        self.write_reg_vx(x, sum as u8);
                        if sum > 0xFF {
                            self.write_reg_vx(0xF, 1);
                        }
                    }
                    5 => {
                        // Vx -= Vy
                        let sub: i8 = vx as i8 - vy as i8;
                        self.write_reg_vx(x, sub as u8);
                        if sub < 0 {
                            self.write_reg_vx(0xFF, 0);
                        }
                        }
                    6 => {
                        // Vx >>= 1
                        self.write_reg_vx(0xF, vx & 0x1);
                        self.write_reg_vx(x, vx >> 1);
                    }
                    _ => panic!("Unrecognized 0x8XY* instruction {:#X}:{:#X}", self.pc, instruction),
               };

               self.pc += 2;
            }

            0xA => {
                // I = NNN
                self.i = nnn;
                self.pc += 2;
            }

            0xD => {
                //draw(Vx,Vy,N)
                let vx = self.read_reg_vx(x);
                let vy = self.read_reg_vx(y);
                self.debug_draw_sprite(bus, vx, vy, n);
                self.pc += 2;
            }

            0xE => {
                match nn {
                    0xA1 => {
                        //if(key()!=Vx) then skip the next instruction
                        let key = self.read_reg_vx(x);
                        if !bus.is_key_pressed(key){
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    0x9E => {
                        //if(key()==Vx) then skip the next instruction
                        let key = self.read_reg_vx(x);
                        if bus.is_key_pressed(key){
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    _ => panic!("Unrecognized 0xEX** instruction {:#X}:{:#X}", self.pc, instruction),
                };

            }

            0xF => {
                match nn {
                    0x07 => {
                        self.write_reg_vx(x, bus.get_delay_timer());
                        self.pc += 2;
                    },
                    0x0A => {
                        // Vx = get_key()
                        let key = bus.get_key_pressed();
                        match key {
                            Some(val) => {
                                self.write_reg_vx(x, val);
                                self.pc += 2;
                            }
                            None => ()
                        }
                    },
                    0x15 => {
                        // delay_timer(Vx)
                        bus.set_delay_timer(self.read_reg_vx(x));
                        self.pc += 2;
                    },
                    0x65 => {
                        // reg_load(Vx, &i)
                        for index in 0..(x+1) {
                            let value = bus.ram_read_byte(self.i + index as u16);
                            self.write_reg_vx(index, value);
                        }
                        self.pc += 2;
                    },
                    0x1E => {    
                        // I += Vx
                        let vx = self.read_reg_vx(x);
                        self.i += vx as u16;
                        self.pc += 2;
                    },
                    _ => panic!("Unrecognized 0xF0** instruction {:#X}:{:#X}", self.pc, instruction),
                }

            }
            _ => panic!("Unrecognized instruction {:#X}:{:#X}", self.pc, instruction),
        }
        
    }

    fn debug_draw_sprite(&mut self, bus: &mut Bus,x: u8, y: u8, height: u8){
        println!("Drawing sprites at ({},{})", x, y);
        let mut should_set_vf = false;
        for sprite_y in 0..height {
            let b = bus.ram_read_byte(self.i + sprite_y as u16);
            if bus.debug_draw_byte(b, x, y + sprite_y) {
                should_set_vf = true;
            }
        }
        if should_set_vf {
            self.write_reg_vx(0xF, 1);
        } else {
            self.write_reg_vx(0xF, 0);
        }
        bus.present_screen();
    }

    pub fn write_reg_vx(&mut self, index: u8, value: u8){
        self.vx[index as usize] = value
    }

    pub fn read_reg_vx(&mut self, index: u8) -> u8{
        self.vx[index as usize]
    }
}

impl fmt::Debug for Cpu{
        
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\npc: {:#X}\n", self.pc)?;
        write!(f, "vx: ")?;
        for item in &self.vx {
            write!(f, "{:#X} ", *item)?;
        }
        write!(f, "\n")?;
        write!(f, "i: {:#X}\n", self.i)
    }
}