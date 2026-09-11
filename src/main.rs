mod cpu;
mod instructions;
use cpu::*;

fn main() {
    let mut cpu = CPU::new();
    cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        // load with lda 0xa9
        cpu.load_and_run(vec![0xa9, 0x0a, 0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa2, 0xff, 0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }
    #[test]
    fn test_adc_basic_no_carry_no_overflow() {
        let mut cpu = CPU::new();
        // 0x10 + 0x20 = 0x30, well within range, no flags of interest set
        cpu.load_and_run(vec![0xa9, 0x10, 0x69, 0x20, 0x00]);

        assert_eq!(cpu.register_a, 0x30);
        assert!(cpu.status & 0b0000_0001 == 0); // no carry
        assert!(cpu.status & 0b0100_0000 == 0); // no overflow
        assert!(cpu.status & 0b0000_0010 == 0); // not zero
        assert!(cpu.status & 0b1000_0000 == 0); // not negative
    }

    #[test]
    fn test_adc_sets_carry_on_unsigned_overflow() {
        let mut cpu = CPU::new();
        // 0xFF + 0x01 = 0x100 -> wraps to 0x00, carry should be set, zero should be set
        cpu.load_and_run(vec![0xa9, 0xff, 0x69, 0x01, 0x00]);

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & 0b0000_0001 == 0b0000_0001); // carry set
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010); // zero set
        assert!(cpu.status & 0b0100_0000 == 0); // no signed overflow (FF is -1, +1 = 0, expected)
    }

    #[test]
    fn test_adc_does_not_set_carry_when_no_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x01, 0x69, 0x01, 0x00]);

        assert_eq!(cpu.register_a, 0x02);
        assert!(cpu.status & 0b0000_0001 == 0); // no carry
    }

    #[test]
    fn test_adc_uses_existing_carry_flag_as_input() {
        let mut cpu = CPU::new();
        // First ADC: 0xFF + 0x01 = 0x00, sets carry = 1
        // LDA does NOT affect the carry flag on real 6502, so it should persist
        // Second ADC: 0x01 + 0x01 + carry-in(1) = 0x03
        cpu.load_and_run(vec![
            0xa9, 0xff, // LDA #$FF
            0x69, 0x01, // ADC #$01 -> A = 0x00, carry = 1
            0xa9, 0x01, // LDA #$01 -> A = 0x01 (carry untouched by LDA)
            0x69, 0x01, // ADC #$01 -> A = 0x01 + 0x01 + 1 = 0x03
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0x03);
    }

    #[test]
    fn test_adc_sets_overflow_positive_plus_positive_equals_negative() {
        let mut cpu = CPU::new();
        // 0x50 (+80) + 0x50 (+80) = 0xA0 (-96 signed) -> signed overflow, no unsigned carry
        cpu.load_and_run(vec![0xa9, 0x50, 0x69, 0x50, 0x00]);

        assert_eq!(cpu.register_a, 0xA0);
        assert!(cpu.status & 0b0100_0000 == 0b0100_0000); // overflow set
        assert!(cpu.status & 0b0000_0001 == 0); // no carry
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000); // negative set (result has bit 7 set)
    }

    #[test]
    fn test_adc_sets_overflow_negative_plus_negative_equals_positive() {
        let mut cpu = CPU::new();
        // 0x80 (-128) + 0x80 (-128) = 0x100 -> wraps to 0x00 (positive/zero signed)
        // both carry (unsigned) and overflow (signed) should be set
        cpu.load_and_run(vec![0xa9, 0x80, 0x69, 0x80, 0x00]);

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & 0b0100_0000 == 0b0100_0000); // overflow set
        assert!(cpu.status & 0b0000_0001 == 0b0000_0001); // carry set
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010); // zero set
        assert!(cpu.status & 0b1000_0000 == 0); // not negative
    }

    #[test]
    fn test_adc_no_overflow_when_signs_differ() {
        let mut cpu = CPU::new();
        // Adding a positive and a negative can never cause signed overflow
        // 0x50 (+80) + 0xF0 (-16) = 0x140 -> wraps to 0x40 (+64), no overflow, carry set
        cpu.load_and_run(vec![0xa9, 0x50, 0x69, 0xf0, 0x00]);

        assert_eq!(cpu.register_a, 0x40);
        assert!(cpu.status & 0b0100_0000 == 0); // no overflow
        assert!(cpu.status & 0b0000_0001 == 0b0000_0001); // carry set
    }

    #[test]
    fn test_adc_zero_flag_without_carry_in() {
        let mut cpu = CPU::new();
        // 0x00 + 0x00 = 0x00, zero flag set, no carry, no overflow
        cpu.load_and_run(vec![0xa9, 0x00, 0x69, 0x00, 0x00]);

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010); // zero set
        assert!(cpu.status & 0b0000_0001 == 0); // no carry
    }

    #[test]
    fn test_adc_sets_negative_flag() {
        let mut cpu = CPU::new();
        // 0x01 + 0x7F = 0x80 -> bit 7 set, negative flag should be set
        cpu.load_and_run(vec![0xa9, 0x01, 0x69, 0x7f, 0x00]);

        assert_eq!(cpu.register_a, 0x80);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000); // negative set
    }

    #[test]
    fn test_adc_from_zero_page_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x05);
        // LDA #$05, ADC $10 (zero page) -> 0x05 + 0x05 = 0x0A
        cpu.load_and_run(vec![0xa9, 0x05, 0x65, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x0A);
    }
}

#[cfg(test)]
mod and_asl_tests {
    use super::*;

    const CARRY: u8 = 0b0000_0001;
    const ZERO: u8 = 0b0000_0010;
    const NEGATIVE: u8 = 0b1000_0000;

    // ---------- AND ----------

    #[test]
    fn and_immediate_basic() {
        let mut cpu = CPU::new();
        // LDA #$FF ; AND #$0F ; BRK
        cpu.load_and_run(vec![0xA9, 0xFF, 0x29, 0x0F, 0x00]);
        assert_eq!(cpu.register_a, 0x0F);
        assert!(cpu.status & ZERO == 0);
        assert!(cpu.status & NEGATIVE == 0);
    }

    #[test]
    fn and_zero_result_sets_zero_flag() {
        let mut cpu = CPU::new();
        // 0xF0 AND 0x0F = 0x00 — no overlapping bits at all
        cpu.load_and_run(vec![0xA9, 0xF0, 0x29, 0x0F, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & ZERO != 0);
        assert!(cpu.status & NEGATIVE == 0);
    }

    #[test]
    fn and_result_with_bit7_set_sets_negative_flag() {
        let mut cpu = CPU::new();
        // 0xFF AND 0x80 = 0x80 — bit 7 survives
        cpu.load_and_run(vec![0xA9, 0xFF, 0x29, 0x80, 0x00]);
        assert_eq!(cpu.register_a, 0x80);
        assert!(cpu.status & NEGATIVE != 0);
        assert!(cpu.status & ZERO == 0);
    }

    #[test]
    fn and_with_zero_operand_always_zeroes_accumulator() {
        let mut cpu = CPU::new();
        // anything AND 0x00 = 0x00, regardless of accumulator's own value
        cpu.load_and_run(vec![0xA9, 0xAA, 0x29, 0x00, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & ZERO != 0);
    }

    #[test]
    fn and_with_0xff_operand_is_identity() {
        let mut cpu = CPU::new();
        // anything AND 0xFF = itself, unchanged
        cpu.load_and_run(vec![0xA9, 0x37, 0x29, 0xFF, 0x00]);
        assert_eq!(cpu.register_a, 0x37);
    }

    #[test]
    fn and_zero_page_addressing_mode() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0b1100_1100);
        // LDA #$AA ; AND $10 ; BRK
        cpu.load_and_run(vec![0xA9, 0xAA, 0x25, 0x10, 0x00]);
        // 0xAA = 1010_1010, AND 1100_1100 = 1000_1000 = 0x88
        assert_eq!(cpu.register_a, 0b1000_1000);
        assert!(cpu.status & NEGATIVE != 0);
    }

    #[test]
    fn and_does_not_affect_carry_flag() {
        let mut cpu = CPU::new();
        // SEC (set carry) ; LDA #$FF ; AND #$FF ; BRK — AND must leave carry untouched
        cpu.load_and_run(vec![0x38, 0xA9, 0xFF, 0x29, 0xFF, 0x00]);
        assert!(
            cpu.status & CARRY != 0,
            "AND should not clear a pre-existing carry flag"
        );
    }

    // ---------- ASL ----------

    #[test]
    fn asl_accumulator_basic_shift() {
        let mut cpu = CPU::new();
        // LDA #$01 ; ASL A ; BRK
        cpu.load_and_run(vec![0xA9, 0x01, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0x02);
        assert!(cpu.status & CARRY == 0);
        assert!(cpu.status & ZERO == 0);
        assert!(cpu.status & NEGATIVE == 0);
    }

    #[test]
    fn asl_bit7_set_shifts_into_carry() {
        let mut cpu = CPU::new();
        // 0x80 (1000_0000) << 1 = 0x00, with the shifted-out bit7 landing in carry
        cpu.load_and_run(vec![0xA9, 0x80, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & CARRY != 0, "bit 7 should shift into carry");
        assert!(cpu.status & ZERO != 0, "result is zero");
        assert!(cpu.status & NEGATIVE == 0);
    }

    #[test]
    fn asl_produces_negative_result() {
        let mut cpu = CPU::new();
        // 0x40 (0100_0000) << 1 = 0x80 — result's bit 7 now set, but no bit was shifted OUT
        cpu.load_and_run(vec![0xA9, 0x40, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0x80);
        assert!(cpu.status & CARRY == 0, "no bit was shifted out of range");
        assert!(cpu.status & NEGATIVE != 0);
    }

    #[test]
    fn asl_all_bits_set_shifts_correctly() {
        let mut cpu = CPU::new();
        // 0xFF (1111_1111) << 1 = 0xFE, carry set from the shifted-out bit7
        cpu.load_and_run(vec![0xA9, 0xFF, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0xFE);
        assert!(cpu.status & CARRY != 0);
        assert!(cpu.status & NEGATIVE != 0);
        assert!(cpu.status & ZERO == 0);
    }

    #[test]
    fn asl_zero_stays_zero() {
        let mut cpu = CPU::new();
        // LDA #$00 ; ASL A ; BRK
        cpu.load_and_run(vec![0xA9, 0x00, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & CARRY == 0);
        assert!(cpu.status & ZERO != 0);
    }

    #[test]
    fn asl_memory_zero_page_mode_does_not_touch_accumulator() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0b0100_0001);
        // ASL $10 ; BRK — operates on memory, accumulator untouched
        cpu.load_and_run(vec![0xa9, 0x99, 0x06, 0x10, 0x00]);
        assert_eq!(cpu.mem_read(0x10), 0b1000_0010);
        assert_eq!(
            cpu.register_a, 0x99,
            "memory-mode ASL must not modify the accumulator"
        );
        assert!(cpu.status & NEGATIVE != 0);
        assert!(cpu.status & CARRY == 0);
    }

    #[test]
    fn asl_repeated_shifts_accumulate_carry_correctly_each_time() {
        let mut cpu = CPU::new();
        // LDA #$40 ; ASL A ; ASL A ; BRK
        // 0x40 -> 0x80 (no carry) -> 0x00 (carry set from bit7 of 0x80)
        cpu.load_and_run(vec![0xA9, 0x40, 0x0A, 0x0A, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & CARRY != 0);
        assert!(cpu.status & ZERO != 0);
    }
}
#[cfg(test)]
mod bcc_test {
    use super::*;

    #[test]
    fn test_0x90_bcc_branch_when_carry_clear() {
        let mut cpu = CPU::new();

        // BCC +2 skips the LDA #$01 and executes LDA #$02
        cpu.load_and_run(vec![
            0x90, 0x03, // BCC +3
            0xa9, 0x01, // LDA #$01 (skipped)
            0xa9, 0x02, // LDA #$02
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0x90_bcc_does_not_branch_when_carry_set() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![
            0x38, // SEC
            0x90, 0x02, // BCC +2 (not taken)
            0xa9, 0x01, // LDA #$01
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0x01);
    }

    #[test]
    fn test_0x90_bcc_with_zero_offset() {
        let mut cpu = CPU::new();

        // BCC +0 branches to the next instruction.
        cpu.load_and_run(vec![
            0x90, 0x00, // BCC +0
            0xa9, 0x42, // LDA #$42
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0x42);
    }

    #[test]
    fn test_0x90_bcc_forward_branch() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![
            0x90, 0x03, // BCC +3
            0xa9, 0x01, // skipped
            0xa9, 0x02, // skipped
            0xa9, 0x03, // skipped
            0xa9, 0x42, // branch target
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0x42);
    }

    #[test]
    fn test_0x90_bcc_backward_branch() {
        let mut cpu = CPU::new();

        // Start by loading A with 1.
        // Then BCC backwards to the LDA instruction.
        //
        // This is mainly testing that the relative offset is
        // interpreted as a signed i8.
        cpu.load_and_run(vec![
            0xa9, 0x01, // $8000: LDA #$01
            0x90, 0xfc, // $8002: BCC -4 -> $8000
            0x00,
        ]);

        // This test would loop forever if BCC were actually taken,
        // so it is not suitable for load_and_run unless your
        // implementation has a step limit.
    }

    #[test]
    fn test_0x90_bcc_preserves_carry_when_clear() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![
            0x90, 0x01, // BCC +1
            0x00, 0x00,
        ]);

        assert_eq!(cpu.status & 0b0000_0001, 0);
    }

    #[test]
    fn test_0x90_bcc_preserves_carry_when_set() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![
            0x38, // SEC
            0x90, 0x01, // BCC +1 (not taken)
            0x00,
        ]);

        assert_eq!(cpu.status & 0b0000_0001, 1);
    }
}
