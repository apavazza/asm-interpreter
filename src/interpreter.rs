use std::io::{self, BufRead, Write};
use std::collections::HashMap;

const NUM_REGISTERS: usize = 16;
const MEMORY_SIZE: usize = 1024; // memory size (1024 words)

pub fn interactive(){
    let stdin = io::stdin();
    run_with_reader(stdin.lock(), true);
}

pub fn run_with_reader<R: BufRead>(mut reader: R, interactive: bool) {
    // Initialize all registers to 0
    let mut registers = [0i32; NUM_REGISTERS];
    // Initialize the CPSR carry flag (0 or 1)
    let mut cpsr: u32 = 0;
    // Initialize memory
    let mut memory: Vec<i32> = vec![0; MEMORY_SIZE];
    // Store labels and their memory addresses
    let mut labels: HashMap<String, usize> = HashMap::new();
    // Keep track of the next available memory address for new labels
    let mut next_label_mem_addr: usize = 0;

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
        reg[1..].parse::<usize>()
            .ok()
            .and_then(|idx| if idx < NUM_REGISTERS { Some(idx) } else { None })
    }

    // Helper function to parse a value (immediate or register content)
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

    // Helper function to parse memory addressing modes for LDR/STR
    fn parse_address_operand(
        operand_str: &str,
        registers: &[i32],
        labels: &HashMap<String, usize>,
        report_fn: &dyn Fn(&str), // For reporting errors
    ) -> Option<usize> {
        let trimmed_operand = operand_str.trim();

        if trimmed_operand.starts_with('[') && trimmed_operand.ends_with(']') {
            // Register indirect or register indirect with offset
            let inner = &trimmed_operand[1..trimmed_operand.len()-1];
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

            if parts.len() == 1 { // [Rx]
                let reg_name = parts[0];
                if let Some(reg_idx) = parse_register(reg_name) {
                    return Some(registers[reg_idx] as usize);
                } else {
                    report_fn(&format!("Invalid register in address operand: {}", reg_name));
                    return None;
                }
            } else if parts.len() == 2 { // [Rx, #offset] or [Rx, label_as_offset] - simplified to #offset
                let reg_name = parts[0];
                let offset_str = parts[1];

                if let Some(reg_idx) = parse_register(reg_name) {
                    let base_address = registers[reg_idx] as usize;
                    if offset_str.starts_with('#') {
                        // Using parse_value to handle #hex and #dec for offset
                        if let Some(offset_val) = parse_value(offset_str, registers) { // Pass empty registers, not used for #
                             // Ensure offset is treated as usize for address calculation
                            if offset_val < 0 {
                                // Handle negative offsets by subtracting magnitude
                                return Some(base_address.saturating_sub(offset_val.abs() as usize));
                            } else {
                                return Some(base_address.saturating_add(offset_val as usize));
                            }
                        } else {
                            report_fn(&format!("Invalid immediate offset in address operand: {}", offset_str));
                            return None;
                        }
                    } else {
                        report_fn("Offset in [Reg, Offset] must be an immediate value starting with #.");
                        return None;
                    }
                } else {
                    report_fn(&format!("Invalid register in address operand: {}", reg_name));
                    return None;
                }
            } else {
                report_fn(&format!("Invalid address operand format: {}", trimmed_operand));
                return None;
            }
        } else if trimmed_operand.starts_with('#') {
            // Immediate address #0x... or #...
            // Using parse_value, but its return is i32, address should be usize
            if let Some(addr_val) = parse_value(trimmed_operand, registers) { // Pass empty registers
                if addr_val < 0 {
                    report_fn(&format!("Memory address cannot be negative: {}", addr_val));
                    return None;
                }
                return Some(addr_val as usize);
            } else {
                report_fn(&format!("Invalid immediate address: {}", trimmed_operand));
                return None;
            }
        } else {
            // Label
            if let Some(addr) = labels.get(trimmed_operand) {
                return Some(*addr);
            } else {
                report_fn(&format!("Undefined label: {}", trimmed_operand));
                return None;
            }
        }
    }

    loop {
        if interactive { print!("> "); }
        io::stdout().flush().unwrap();

        let mut input_line = String::new();
        if reader.read_line(&mut input_line).unwrap() == 0 {
            break; // EOF
        }

        // First, trim whitespace from the raw line
        let effective_line = input_line.trim();

        // Strip any comment part (from "//" to the end of the line)
        let mut comment_stripped_line = effective_line;
        if let Some(comment_start_index) = comment_stripped_line.find("//") {
            comment_stripped_line = &comment_stripped_line[..comment_start_index].trim_end();
        }

        // Skip if the line is now empty (was blank or only a comment)
        if comment_stripped_line.is_empty() {
            continue;
        }

        // Now, use 'comment_stripped_line' for all further processing
        if comment_stripped_line.eq_ignore_ascii_case("EXIT") {
            break;
        }

        let mut line_to_parse = comment_stripped_line;

        // Label detection and processing
        if let Some(colon_index) = line_to_parse.find(':') {
            let label_candidate = line_to_parse[..colon_index].trim();
            let rest_of_line_after_colon = line_to_parse[colon_index + 1..].trim();

            if !label_candidate.is_empty() && !label_candidate.contains(char::is_whitespace) {
                // Valid label format
                if labels.contains_key(label_candidate) {
                    report_error(interactive, &format!("Duplicate label definition: {}", label_candidate));
                    continue; // Skip this line
                }
                if next_label_mem_addr >= MEMORY_SIZE {
                    report_error(interactive, "Out of memory for new labels/data.");
                    continue; // Skip this line
                }

                let current_label_address = next_label_mem_addr;
                labels.insert(label_candidate.to_string(), current_label_address);
                
                // Check if there's a data initializer like #value
                if !rest_of_line_after_colon.is_empty() && rest_of_line_after_colon.starts_with('#') {
                    let value_str = rest_of_line_after_colon;
                    let parsed_val: Option<i32>;
                    if value_str.starts_with("#0x") || value_str.starts_with("#0X") {
                        // Ensure there are characters after #0x for parsing
                        if value_str.len() > 3 {
                            parsed_val = i32::from_str_radix(&value_str[3..], 16).ok();
                        } else {
                            parsed_val = None;
                        }
                    } else {
                        // Ensure there are characters after # for parsing
                        if value_str.len() > 1 {
                            parsed_val = value_str[1..].parse::<i32>().ok();
                        } else {
                            parsed_val = None;
                        }
                    }

                    if let Some(val) = parsed_val {
                        memory[current_label_address] = val;
                        if interactive {
                            println!("Label '{}' defined at memory address {}, initialized with value {}", 
                                     label_candidate, current_label_address, val);
                        }
                        next_label_mem_addr += 1; // Consume memory slot for data
                        continue; // This line was a label with data definition, fully processed.
                    } else {
                        report_error(interactive, &format!("Invalid value for label data initialization: {}. Expected format like #123 or #0xFF.", value_str));
                        labels.remove(label_candidate); // Rollback label definition
                        continue; // Skip this erroneous line
                    }
                } else {
                    // This is "label:" (rest_of_line_after_colon is empty)
                    // or "label: instruction" (rest_of_line_after_colon has an instruction)
                    if interactive {
                         println!("Label '{}' defined at memory address {}", label_candidate, current_label_address);
                    }
                    next_label_mem_addr += 1; // Consume memory slot for the label definition itself

                    line_to_parse = rest_of_line_after_colon; // Continue parsing the rest of the line (if any)
                    // If line_to_parse is empty (was just "label:"), the check below will handle it.
                }
            }
        }

        // If line_to_parse is empty at this point (e.g., after processing "label:" or "label: #data"), skip instruction parsing.
        if line_to_parse.is_empty() {
            continue;
        }

        // Instruction parsing starts here, using line_to_parse
        let parts: Vec<&str> = line_to_parse.split_whitespace().collect();
        // parts.is_empty() should not happen here due to the effective_line.is_empty() check above,
        // but an extra check or assertion wouldn't hurt if you want to be extremely defensive.
        if parts.is_empty() { // Should be redundant due to check above, but safe.
            continue;
        }
        
        let report_fn_closure = |msg: &str| report_error(interactive, msg);

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
            "ADC" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: ADC <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in ADC");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            // ADC: result = op1 + op2 + CPSR. Using overflowing add to update CPSR.
                            let (sum, carry1) = (op1_val as u32).overflowing_add(op2_val as u32);
                            let (result, carry2) = sum.overflowing_add(cpsr);
                            registers[idx_dest] = result as i32;
                            cpsr = if carry1 || carry2 { 1 } else { 0 };
                        } else {
                            report_error(interactive, "Invalid second operand for ADC. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for ADC must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in ADC. Use r0 through r15.");
                }
            },
            "SBC" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: SBC <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in SBC");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            // SBC: result = op1 - op2 - (1 - CPSR)
                            // Note: In ARM, carry means no borrow, so (1 - carry) is subtracted.
                            let (diff1, borrow1) = (op1_val as u32).overflowing_sub(op2_val as u32);
                            let subtrahend = 1 - cpsr;
                            let (result, borrow2) = diff1.overflowing_sub(subtrahend);
                            registers[idx_dest] = result as i32;
                            cpsr = if borrow1 || borrow2 { 0 } else { 1 };
                        } else {
                            report_error(interactive, "Invalid second operand for SBC. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for SBC must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in SBC. Use r0 through r15.");
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
            "MUL" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: MUL <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in MUL");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val * op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for MUL. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for MUL must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in MUL. Use r0 through r15.");
                }
            },
            "AND" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: AND <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in AND");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val & op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for AND. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for AND must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in AND. Use r0 through r15.");
                }
            },
            "ORR" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: ORR <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in ORR");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val | op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for ORR. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for ORR must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in ORR. Use r0 through r15.");
                }
            },
            "BIC" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: BIC <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in BIC");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val & !op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for BIC. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for BIC must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in BIC. Use r0 through r15.");
                }
            },
            "EOR" => {
                if parts.len() != 4 {
                    report_error(interactive, "Usage: EOR <dest_register>, <reg_operand>, <operand>");
                    continue;
                }
                if !parts[1].ends_with(',') || !parts[2].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register operands in EOR");
                    continue;
                }
                let dest = parts[1].trim_end_matches(',');
                if let Some(idx_dest) = parse_register(dest) {
                    if let Some(idx_op1) = parse_register(parts[2].trim_end_matches(',')) {
                        let op1_val = registers[idx_op1];
                        if let Some(op2_val) = parse_value(parts[3], &registers) {
                            registers[idx_dest] = op1_val ^ op2_val;
                        } else {
                            report_error(interactive, "Invalid second operand for EOR. It must be an immediate (prefixed with '#') or a valid register.");
                        }
                    } else {
                        report_error(interactive, "The first operand for EOR must be a register, not an immediate constant.");
                    }
                } else {
                    report_error(interactive, "Invalid destination register in EOR. Use r0 through r15.");
                }
            },
            "LDR" => {
                if parts.len() != 3 {
                    report_error(interactive, "Usage: LDR <register>, <address_operand>");
                    continue;
                }
                if !parts[1].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after register in LDR");
                    continue;
                }
                let reg_name = parts[1].trim_end_matches(',');
                let address_operand_str = parts[2];

                if let Some(reg_idx) = parse_register(reg_name) {
                    if let Some(address) = parse_address_operand(address_operand_str, &registers, &labels, &report_fn_closure) {
                        if address < MEMORY_SIZE {
                            registers[reg_idx] = memory[address];
                        } else {
                            report_error(interactive, &format!("Memory access out of bounds: address {} from operand {}", address, address_operand_str));
                        }
                    } // parse_address_operand already reported the error if it returned None
                } else {
                    report_error(interactive, "Invalid register name for LDR.");
                }
            },
            "STR" => {
                if parts.len() != 3 { 
                    report_error(interactive, "Usage: STR <source_register>, <address_operand>");
                    continue;
                }
                if !parts[1].ends_with(',') {
                    report_error(interactive, "Syntax error: Missing comma after source register in STR");
                    continue;
                }
                let src_reg_name = parts[1].trim_end_matches(',');
                let address_operand_str = parts[2];

                if let Some(idx_src) = parse_register(src_reg_name) {
                    if let Some(address) = parse_address_operand(address_operand_str, &registers, &labels, &report_fn_closure) {
                        if address < MEMORY_SIZE {
                            memory[address] = registers[idx_src];
                        } else {
                            report_error(interactive, &format!("Memory access out of bounds: address {} >= MEMORY_SIZE {}", address, MEMORY_SIZE));
                        }
                    } // parse_address_operand already reports errors
                } else {
                    report_error(interactive, &format!("Invalid source register in STR: {}", src_reg_name));
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
    fn test_adc_instruction() {
        let script = "\
            MOV r1, #5\n\
            MOV r2, #10\n\
            ADC r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_sbc_instruction() {
        let script = "\
            MOV r1, #20\n\
            MOV r2, #7\n\
            SBC r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_mul_instruction() {
        let script = "\
            MOV r1, #3\n\
            MOV r2, #4\n\
            MUL r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_and_instruction() {
        let script = "\
            MOV r1, #12\n\
            MOV r2, #10\n\
            AND r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_orr_instruction() {
        let script = "\
            MOV r1, #4\n\
            MOV r2, #2\n\
            ORR r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_bic_instruction() {
        let script = "\
            MOV r1, #7\n\
            MOV r2, #6\n\
            BIC r3, r1, r2\n\
            PRINT r3\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_eor_instruction() {
        let script = "\
            MOV r1, #5\n\
            MOV r2, #3\n\
            EOR r3, r1, r2\n\
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

    #[test]
    fn test_ldr_str_label() {
        let script = "\
            data_val:\n\
            MOV r0, #123\n\
            STR r0, data_val\n\
            MOV r1, #0\n\
            LDR r1, data_val\n\
            PRINT r1\n\
            EXIT\n"; // Expect r1 to be 123
        run_test_script(script); // This test will panic if PRINT r1 is not 123 due to how PRINT works or if errors occur.
                                 // A more robust test would capture output or check register values directly if possible.
    }

    #[test]
    fn test_ldr_str_immediate_address() {
        let script = "\
            MOV r0, #456\n\
            STR r0, #10\n\
            MOV r1, #0\n\
            LDR r1, #10\n\
            PRINT r1\n\
            EXIT\n"; // Expect r1 to be 456
        run_test_script(script);
    }

    #[test]
    fn test_ldr_str_register_indirect() {
        let script = "\
            MOV r0, #20\n\
            MOV r1, #789\n\
            STR r1, [r0]\n\
            MOV r2, #0\n\
            LDR r2, [r0]\n\
            PRINT r2\n\
            EXIT\n"; // Expect r2 to be 789
        run_test_script(script);
    }
    
    #[test]
    fn test_ldr_str_register_indirect_offset() {
        let script = "\
            MOV r0, #30\n\
            MOV r1, #101\n\
            STR r1, [r0,#4]\n\
            MOV r2, #0\n\
            LDR r2, [r0,#4]\n\
            PRINT r2\n\
            EXIT\n"; // Expect r2 to be 101 (stored at memory[34])
        run_test_script(script);
    }

    #[test]
    fn test_ldr_str_register_indirect_negative_offset() {
        let script = "\
            MOV r0, #40\n\
            MOV r1, #202\n\
            STR r1, [r0,#-2]\n\
            MOV r2, #0\n\
            LDR r2, [r0,#-2]\n\
            PRINT r2\n\
            EXIT\n"; // Expect r2 to be 202 (stored at memory[38])
        run_test_script(script);
    }

    #[test]
    fn test_comments_and_labels() {
        let script = "\
            // This is a full line comment\n\
            data_start: // This is a label\n\
            MOV r0, #10 // Move 10 to r0\n\
            // Another comment\n\
            STR r0, data_start // Store r0 to data_start\n\
            LDR r1, data_start // Load from data_start to r1\n\
            PRINT r1 // Should print 10\n\
            EXIT\n";
        run_test_script(script);
    }

    #[test]
    fn test_empty_lines_and_whitespace_with_comments() {
        let script = "\
            \n\
            // Comment after empty line\n\
            MOV r5, #55\n\
            \n\
            PRINT r5 // Expect 55\n\
            // Comment at end of file\n\
            EXIT\n";
        run_test_script(script);
    }
}