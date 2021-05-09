use std::{
    env,
    fs::File,
    io::{prelude::*, BufReader, BufWriter},
    path::Path,
};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

fn build_ascii_font() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();

    let input_path = Path::new("assets/ascii_font.txt");
    let output_path = Path::new(&out_dir).join("ascii_font.rs");

    println!("cargo:rerun-if-changed={}", input_path.display());

    let input = File::open(input_path)?;
    let input = BufReader::new(input);
    let output = File::create(output_path)?;
    let mut output = BufWriter::new(output);

    writeln!(
        &mut output,
        "pub(crate) const ASCII_FONT: [[u8; 16]; 256] = ["
    )?;

    let mut last_index = None;
    let mut lines = input.lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("0x") {
            let (index_str, ch_str) = rest.split_at(2);
            let index = usize::from_str_radix(index_str, 16).unwrap();
            assert!(index == 0 || Some(index - 1) == last_index);
            last_index = Some(index);

            writeln!(&mut output, "    // 0x{}{}", index_str, ch_str)?;
            writeln!(&mut output, "    [")?;
            for line in lines.by_ref() {
                let line = line?;
                let line = line.trim();
                if !line.starts_with(&['.', '@'][..]) {
                    break;
                }
                let mut output_num = 0;
                for ch in line.chars() {
                    let bit = match ch {
                        '.' => 0,
                        '@' => 1,
                        _ => panic!("invalid char: {:?}", ch),
                    };
                    output_num = (output_num << 1) | bit;
                }
                writeln!(&mut output, "        0b{:08b},", output_num)?;
            }
            writeln!(&mut output, "    ],")?;
        }
    }

    writeln!(&mut output, "];")?;

    Ok(())
}

fn main() -> Result<()> {
    build_ascii_font()?;
    Ok(())
}
