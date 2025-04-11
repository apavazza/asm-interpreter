use std::io::{self, BufRead, Write};

const NUM_REGISTERS: usize = 16;

pub fn interactive(){
    let stdin = io::stdin();
    run_with_reader(stdin.lock(), true);
}

pub fn run_with_reader<R: BufRead>(mut reader: R, interactive: bool) {
    // Initialize all registers to 0
    let mut registers = [0i32; NUM_REGISTERS];

    fn report_error(interactive: bool, msg: &str) {
        if interactive {
            println!("{}", msg);
        } else {
            panic!("{}", msg);
        }
    }

    // Helper function to parse a register name
    fn parse_register(reg: &str) -> Option<usize> {
        if reg.len() < 2 || !reg.to_lowercase().starts_with('r') {
            return None;
        }
        reg[1..].parse::<usize>().ok().and_then(|idx| if idx < NUM_REGISTERS { Some(idx) } else { None })
    }

    // Helper function to parse a value
    fn parse_value(s: &str, registers: &[i32]) -> Option<i32> {
        if s.starts_with('#') {
            let imm_str = &s[1..];
            // Support hexadecimal if prefixed with "0x" (or "0X")s
            if imm_str.starts_with("0x") || imm_str.starts_with("0X") {
                i32::from_str_radix(&imm_str[2..], 16).ok()
            } else {
                imm_str.parse::<i32>().ok()
            }
        } else {
            // Otherwise, assume it's a register and return its current value
            parse_register(s).map(|idx| registers[idx])
        }
    }

    loop {
        if interactive { print!("> "); }
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if reader.read_line(&mut input).unwrap() == 0 {
            break;
        }
        let input = input.trim();
        if input.eq_ignore_ascii_case("EXIT") {
            break;
        }
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0].to_uppercase().as_str() {
            "MOV" => {
                if parts.len() != 3 {
                    report_error(interactive, "Usage: MOV <register>, <value>");
                    continue;
                }
                if !parts[1].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register in MOV");
                    continue;
                }
                let reg_name = parts[1].trim_end_matches(',');
                if let Some(idx) = parse_register(reg_name) {
                    if let Some(val) = parse_value(parts[2], &registers) {
                        registers[idx] = val;
                    } else {
                        report_error(interactive, "Invalid operand for MOV. Use immediate with '#' (e.g. \"#0x10\" or \"#15\") or a valid register.");
                    }
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "ADD" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: ADD <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in ADD");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    // The first operand must be a register
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        // The second operand may be an immediate or a register
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val + op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for ADD. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for ADD must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register. Use r0 through r15.");
                }
            },
            "SUB" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: SUB <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in SUB");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val - op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for SUB. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for SUB must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register. Use r0 through r15.");
                }
            },
            "LSL" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: LSL <dest_register>, <source_register>, <shift_amount>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma in LSL instruction");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                let src = parts[2].trim_end_matches(',');
                if let (Some(idx_dest), Some(idx_src)) = (parse_register(dest), parse_register(src)) {
                    if let Some(shift_val) = parse_value(parts[3], &registers) {
                        registers[idx_dest] = registers[idx_src] << (shift_val as u32);
                    } else {
                        report_error(interactive, "Invalid shift amount for LSL instruction.");
                    }
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "LSR" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: LSR <dest_register>, <source_register>, <shift_amount>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma in LSR instruction");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                let src = parts[2].trim_end_matches(',');
                if let (Some(idx_dest), Some(idx_src)) = (parse_register(dest), parse_register(src)) {
                    if let Some(shift_val) = parse_value(parts[3], &registers) {
                        registers[idx_dest] = ((registers[idx_src] as u32) >> (shift_val as u32)) as i32;
                    } else {
                        report_error(interactive, "Invalid shift amount for LSR instruction.");
                    }
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "ASR" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: ASR <dest_register>, <source_register>, <shift_amount>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma in ASR instruction");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                let src = parts[2].trim_end_matches(',');
                if let (Some(idx_dest), Some(idx_src)) = (parse_register(dest), parse_register(src)) {
                    if let Some(shift_val) = parse_value(parts[3], &registers) {
                        registers[idx_dest] = registers[idx_src] >> (shift_val as u32);
                    } else {
                        report_error(interactive, "Invalid shift amount for ASR instruction.");
                    }
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "ROR" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: ROR <dest_register>, <source_register>, <rotate_amount>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma in ROR instruction");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                let src = parts[2].trim_end_matches(',');
                if let (Some(idx_dest), Some(idx_src)) = (parse_register(dest), parse_register(src)) {
                    if let Some(rotate_val) = parse_value(parts[3], &registers) {
                        registers[idx_dest] = (registers[idx_src] as u32).rotate_right(rotate_val as u32) as i32;
                    } else {
                        report_error(interactive, "Invalid rotate amount for ROR instruction.");
                    }
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "RRX" => {
                if parts.len() != 3 {
                    report_error(interactive, "Usage: RRX <dest_register>, <source_register>");
                    continue;
                }
                if !parts[1].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after destination register in RRX");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                let src = parts[2];
                if let (Some(idx_dest), Some(idx_src)) = (parse_register(dest), parse_register(src)) {
                    registers[idx_dest] = ((registers[idx_src] as u32) >> 1) as i32;
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            "PRINT" => {
                if parts.len() != 2 {
                    report_error(interactive, "Usage: PRINT <register>");
                    continue;
                }
                let reg = parts[1];
                if let Some(idx) = parse_register(reg) {
                    println!("{} = {}", reg, registers[idx]);
                } else {
                    report_error(interactive, "Invalid register name. Use r0 through r15.");
                }
            },
            _ => {
                report_error(interactive, &format!("Unknown instruction: {}", parts[0]));
                if !interactive {
                    println!("Exiting due to unknown instruction.");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A helper that runs the interpreter with a given script
    fn run_test_script(script: &str) {
        let input = script.as_bytes();
        let cursor = Cursor::new(input);
        run_with_reader(cursor, false);
    }

    #[test]
    fn test_add_instruction() {
        let script = "\
            MOV r1, #5\n\
            MOV r2, #10\n\
            ADD r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_sub_instruction() {
        let script = "\
            MOV r1, #20\n\
            MOV r2, #7\n\
            SUB r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_lsl_instruction() {
        let script = "\
            MOV r1, #1\n\
            LSL r2, r1, #3\n\
            PRINT r2\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_lsr_instruction() {
        let script = "\
            MOV r1, #16\n\
            LSR r2, r1, #2\n\
            PRINT r2\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_asr_instruction() {
        let script = "\
            MOV r1, #-32\n\
            ASR r2, r1, #2\n\
            PRINT r2\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_ror_instruction() {
        let script = "\
            MOV r1, #4\n\
            ROR r2, r1, #1\n\
            PRINT r2\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_rrx_instruction() {
        let script = "\
            MOV r1, #8\n\
            RRX r2, r1\n\
            PRINT r2\n\
            EXIT\n";
        run_test_script(script);
    }
}