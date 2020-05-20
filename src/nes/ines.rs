// http://fms.komkon.org/EMUL8/NES.html#LABM
// https://formats.kaitai.io/ines/index.html
//
extern crate nom;

use nom::{
    bytes::complete::tag, cond, error::make_error, error::ErrorKind, number::complete::be_u8, take,
    Err, IResult,
};

const MAGIC: &[u8] = b"NES\x1A";
const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;
const PRG_RAM_PAGE_SIZE: usize = 8192;

#[derive(Debug)]
pub struct Rom {
    pub trainer: Option<Vec<u8>>,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub tv_format: TVFormat,
    pub ram_size: usize,
    pub rom_flags: RomFlags,
}

#[derive(Debug)]
pub enum TVFormat {
    PAL,
    NTSC,
}

bitflags! {
    pub struct RomFlags: u8 {
        const VERTICAL_MIRRORING = 0b00000001;
        const BATTERY_RAM        = 0b00000010;
        const TRAINER            = 0b00000100;
        const FOUR_SCREEN        = 0b00001000;
    }
}

impl Rom {
    // Byte     Contents
    // ---------------------------------------------------------------------------
    // 0-3      String "NES^Z" used to recognize .NES files.
    // 4        Number of 16kB ROM banks.
    // 5        Number of 8kB VROM banks.
    // 6        bit 0     1 for vertical mirroring, 0 for horizontal mirroring.
    //          bit 1     1 for battery-backed RAM at $6000-$7FFF.
    //          bit 2     1 for a 512-byte trainer at $7000-$71FF.
    //          bit 3     1 for a four-screen VRAM layout.
    //          bit 4-7   Four lower bits of ROM Mapper Type.
    // 7        bit 0     1 for VS-System cartridges.
    //          bit 1-3   Reserved, must be zeroes!
    //          bit 4-7   Four higher bits of ROM Mapper Type.
    // 8        Number of 8kB RAM banks. For compatibility with the previous
    //          versions of the .NES format, assume 1x8kB RAM page when this
    //          byte is zero.
    // 9        bit 0     1 for PAL cartridges, otherwise assume NTSC.
    //          bit 1-7   Reserved, must be zeroes!
    // 10-15    Reserved, must be zeroes!
    // 16-...   ROM banks, in ascending order. If a trainer is present, its
    //          512 bytes precede the ROM bank contents.
    // ...-EOF  VROM banks, in ascending order.
    // ---------------------------------------------------------------------------
    //
    // TODO: vs_unisistem
    fn _load(input: &[u8]) -> IResult<&[u8], Rom> {
        let (input, _) = tag(MAGIC)(input)?;
        let (input, len_prg_rom) = be_u8(input)?;
        let (input, len_chr_rom) = be_u8(input)?;
        let (input, _byte6) = be_u8(input)?;

        let rom_flags = RomFlags::from_bits(0b000001111 & _byte6).unwrap(); //cant' fail

        let (input, byte7) = be_u8(input)?;
        let _vs_unisystem = byte7 & 1;

        if byte7 & 0x0C == 0x08 {
            return Err(Err::Failure(make_error(input, ErrorKind::OneOf)));
        }

        let mapper = byte7 & 0b11110000 | (_byte6 >> 4);

        let (input, len_ram_banks) = be_u8(input)?;

        let (input, byte9) = be_u8(input)?;
        let pal = byte9 & 1;

        let (input, _) = take!(input, 6)?;

        let (input, trainer) = cond!(input, rom_flags.contains(RomFlags::TRAINER), take!(512))?;

        let (input, prg_rom) = take!(input, PRG_ROM_PAGE_SIZE * len_prg_rom as usize)?;
        let (input, chr_rom) = take!(input, CHR_ROM_PAGE_SIZE * len_chr_rom as usize)?;
        Ok((
            input,
            Rom {
                trainer: trainer.map(|t| t.to_vec()),
                prg_rom: prg_rom.to_vec(),
                chr_rom: chr_rom.to_vec(),
                mapper: mapper,
                tv_format: (if pal == 1 {
                    TVFormat::PAL
                } else {
                    TVFormat::NTSC
                }),
                ram_size: PRG_RAM_PAGE_SIZE * len_ram_banks as usize,
                rom_flags: rom_flags,
            },
        ))
    }

    pub fn load(input: &[u8]) -> Result<Rom, &str> {
        match Rom::_load(input) {
            IResult::Ok((_, rom)) => Result::Ok(rom),
            IResult::Err(nom::Err::Error((_, _kind))) => Result::Err("failed to read file"),
            IResult::Err(nom::Err::Failure((_, kind))) if kind == ErrorKind::OneOf => {
                Result::Err("NES2.0 format is not supported")
            }
            IResult::Err(nom::Err::Failure((_, _kind))) => Result::Err("failed to read file"),
            IResult::Err(nom::Err::Incomplete(_)) => Result::Err("Unexpected end of file"),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        pgp_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }

    fn create_rom(rom: TestRom) -> Vec<u8> {
        let mut result = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().map_or(0, |t| t.len())
                + rom.pgp_rom.len()
                + rom.chr_rom.len(),
        );

        result.extend(&rom.header);
        if let Some(t) = rom.trainer {
            result.extend(t);
        }
        result.extend(&rom.pgp_rom);
        result.extend(&rom.chr_rom);

        result
    }

    #[test]
    fn test() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        let rom: Rom = Rom::load(&test_rom).unwrap();

        assert_eq!(rom.trainer, None);
        assert_eq!(rom.chr_rom, vec!(2; 1 * CHR_ROM_PAGE_SIZE));
        assert_eq!(rom.prg_rom, vec!(1; 2 * PRG_ROM_PAGE_SIZE));
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.ram_size, 0);
        assert_eq!(rom.rom_flags.bits, 0b0001);
    }

    #[test]
    fn test_broken() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1],
        });

        let rom = Rom::load(&test_rom);
        match rom {
            Result::Ok(_) => assert!(false, "should not load rom"),
            Result::Err(str) => assert_eq!(str, "Unexpected end of file"),
        }
    }

    #[test]
    fn test_nes2_is_not_supported() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x31, 0x8, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: vec![1; 1 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });
        let rom = Rom::load(&test_rom);
        match rom {
            Result::Ok(_) => assert!(false, "should not load rom"),
            Result::Err(str) => assert_eq!(str, "NES2.0 format is not supported"),
        }
    }
}
