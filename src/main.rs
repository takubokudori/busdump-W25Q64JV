use anyhow::Result;
use serialport::{Parity, SerialPort};
use std::{
    io::{Read, Write},
    time::Instant,
};

fn write_pirate_cmd(sp: &mut Box<dyn SerialPort>, s: &str) -> Result<()> {
    println!("write!: {}", s);
    for x in s.as_bytes().chunks(2) {
        sp.write_all(x)?;
        sp.flush()?;
        wait(1);
    }
    Ok(())
}

fn wait(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

fn read_pirate_cmd(sp: &mut Box<dyn SerialPort>) -> Result<String> {
    let b = sp.bytes_to_read()? as usize;
    let mut v = Vec::with_capacity(b);
    unsafe {
        v.set_len(b);
    }
    sp.read_exact(&mut v)?;
    Ok(String::from_utf8(v)?)
}

fn write_read_spi_cmd(
    sp: &mut Box<dyn SerialPort>,
    s: &str,
    millis: u64,
) -> Result<Vec<String>> {
    write_pirate_cmd(sp, s)?;
    wait(millis);
    Ok(read_pirate_cmd(sp)?
        .lines()
        .map(|x| x.to_string())
        .collect())
}

fn write_read_spi_cmd2(
    sp: &mut Box<dyn SerialPort>,
    s: &str,
) -> Result<Vec<String>> {
    write_pirate_cmd(sp, s)?;
    let mut r = "".to_string();
    let mut i = 0;
    while i < 10 {
        wait(300);
        let rp = read_pirate_cmd(sp)?;
        println!("rp: {}, r: {}, r/5: {}", rp.len(), r.len(), r.len() / 5);
        if rp.is_empty() {
            i += 1;
        } else {
            i = 0;
        }
        r += &rp;
    }
    Ok(r.lines().map(|x| x.to_string()).collect())
}

fn find_parse_read(s: &[String]) -> Option<Vec<u8>> {
    for x in s.iter() {
        if x.starts_with("READ: ") {
            return parse_read(x);
        }
    }
    None
}

fn parse_read(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("READ: ")?;
    let s = match s.strip_suffix(' ') {
        Some(x) => x,
        None => return Some(vec![]),
    };
    let mut ret = Vec::with_capacity(s.len() / 5 + 1);
    for x in s.split(' ') {
        let x = x.strip_prefix("0x")?;
        let u = u8::from_str_radix(x, 16).expect("Unknown!!");
        ret.push(u);
    }
    Some(ret)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut sp = serialport::new(&args[1], 115200)
        .parity(Parity::None)
        .open()
        .expect("Failed to open port");
    let x = write_read_spi_cmd(&mut sp, "\nm\n5\n4\n\n\n\n\n2\n", 1000)
        .expect("failed1");
    println!("x: {:?}", x);
    assert_eq!(&x[x.len() - 2], "Ready");

    let x = write_read_spi_cmd(&mut sp, "W\n", 500).expect("f2");
    assert_eq!(&x[1], "POWER SUPPLIES ON");
    let x = write_read_spi_cmd2(&mut sp, "[0x9f r:3]\n").expect("f3");
    println!("{:?}", x);
    println!("find_parse_read: {:X?}", find_parse_read(&x));

    let mut data = vec![];
    let now = Instant::now();
    for cur in (0..0x50_0000).step_by(65535) {
        let c = format!("{:06X}", cur);
        let elapsed = now.elapsed();
        println!(
            "{}m{}s: try 0x{}",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60,
            c
        );
        let x = write_read_spi_cmd2(
            &mut sp,
            &format!("[0x03 0x{} 0x{} 0x{} r:65535]\n",
                &c[0..2],
                &c[2..4],
                &c[4..6]
            ),
        )
        .expect("f4");
        let fpr = find_parse_read(&x).expect("f5");
        std::fs::write(format!("output/out{}.bin", cur), &fpr).unwrap();
        data.extend(fpr);
    }
    std::fs::write("out.bin", data).unwrap();
}
