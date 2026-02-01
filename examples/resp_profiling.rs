// use rustis::resp::RespDeserializer;
// use serde::Deserialize;

fn main() -> rustis::Result<()> {
    // let mut raw_data = Vec::new();
    // raw_data.extend_from_slice(b"$1024\r\n");
    // raw_data.extend_from_slice(&vec![b'A'; 1024]);
    // raw_data.extend_from_slice(b"\r\n");

    // println!("desrializer stress-test startup...");

    // for i in 0..500_000 {
    //     let (frame, _) = RespFrameParser::new(&raw_data).parse()?;
    //     let response = RespResponse::new(RespBuf::from(Bytes::copy_from_slice(buf)), frame);
    //     let mut resp_deserializer = RespDeserializer::new(&raw_data);
    //     let result = String::deserialize(&mut resp_deserializer);
    //     let _ = std::hint::black_box(result);

    //     if i % 100_000 == 0 {
    //         println!("Processed {} iterations", i);
    //     }
    // }

    Ok(())
}
