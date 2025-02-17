use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::{audio::{SampleBuffer, SignalSpec}, codecs::DecoderOptions, errors::Error::IoError, formats::FormatOptions, io::{MediaSourceStream, MediaSourceStreamOptions}, meta::MetadataOptions, probe::Hint};

fn main() {
    let dir = dbg!(std::env::home_dir().unwrap().join("Desktop/mc")).read_dir().expect("dir not found");
    let mut file_path = None;
    for entry in dir {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            file_path = Some(entry.path());
            break
        }
    }
    
    let file = std::fs::File::open(file_path.expect("No files in dir")).expect("can't open rip");
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions { buffer_len: 64 * 1024 });
    
    let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &FormatOptions::default(), &MetadataOptions::default()).expect("uh oh bad format");
    let mut format = probe.format;
    let track = format.default_track().unwrap();
    dbg!(track.codec_params.channels, track.codec_params.sample_rate, track.codec_params.n_frames, track.codec_params.bits_per_sample);
    
    let mut sample_buf = SampleBuffer::<f32>::new(track.codec_params.n_frames.unwrap(), SignalSpec { rate: track.codec_params.sample_rate.unwrap(), channels: track.codec_params.channels.unwrap()});
    
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).expect("codec not supported");
    
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(IoError(_)) => break,
            Err(_) => panic!(),
        };
        sample_buf.append_interleaved_ref(decoder.decode(&packet).expect("Packet decoding error"));
    }
    
    
    
    let audio_host = cpal::default_host();
    let device = audio_host.default_output_device().expect("No default device found");
    let mut supported_configs_range = device.supported_output_configs().expect("No device configs found");
    
    let mut config = None;
    while let Some(c) = supported_configs_range.next() {
        if c.sample_format() == cpal::SampleFormat::F32 {
            config = Some(c);
            break
        }
    }
    let config = config.expect("No F32 sample format available").with_sample_rate(cpal::SampleRate(48000));
    
    let mut c = 0;
    let (tx, rx) = std::sync::mpsc::channel();
    
    let stream = device.build_output_stream(&config.into(), move |data: &mut [f32], _output_callback_info: &cpal::OutputCallbackInfo| {
        if sample_buf.len() - c >= data.len() {
            data.copy_from_slice(&sample_buf.samples()[c..(c + data.len())]);
        } else {
            let n_left = sample_buf.len() - c;
            data[..n_left].copy_from_slice(&sample_buf.samples()[c..]);
            tx.send(()).unwrap();
        }
        c += data.len();
    }, move |err| { println!("{err}"); }, None).unwrap();
    
    stream.play().unwrap();
    rx.recv().unwrap();
    
    println!("Done!");
}
