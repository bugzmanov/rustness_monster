extern crate sdl2; 

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use rustness::screen::frame;
use sdl2::rect::Point;
use std::time::{SystemTime};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("rust-sdl2 demo", 400, 300)
        .position_centered()
        .build()
        .unwrap();

    
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
 
    // canvas.set_draw_color(Color::RGB(0, 255, 255));
    // canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut frame = frame::Frame::new();

    for y in 1 .. 240 {
        for x in 1 .. 256 {
            frame.set_pixel(x-1, y-1, ((x-1) as u8, 0, 0));
        }
    }

    let mut lloop = 1;
    
    let mut prev_time = SystemTime::now();
    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 320, 240).unwrap();

    'running: loop {
        // let elapsed = SystemTime::now().duration_since(prev_time).unwrap().as_nanos();
        // println!("fps: {}", elapsed );
        // canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        // canvas.set_draw_color(Color::RGB(0, 255, 255));
        // canvas.clear();

        // for z in (0 .. frame.data.len()).step_by(3) {
        //     let x = 10 + z /3 % 256 ;
        //     let y = 10 + z/3 / 256;
        //     let (r,g,b)  = (frame.data[z].wrapping_add(lloop),frame.data[z+1],frame.data[z+2]);
        //     let rgba = [r.wrapping_add(1),g,b];
        //     // let byte = u32::from_be_bytes(rgba);
        //     texture.update(Some(Rect::new(x as i32,y as i32,1,1,)), &rgba,3);


        //     // canvas.set_draw_color(Color::RGB(r, g, b));
        //     // canvas.draw_point(Point::new(x as i32,y as i32));
        // }
        texture.update(Some(Rect::new(0 as i32,0 as i32, 256, 240,)), &frame.data, lloop*3);


        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...


        canvas.clear();
        canvas.copy(&texture, None, Some(Rect::new(10, 20, 266, 250))).unwrap();


        canvas.present();
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        lloop +=1;
        let elapsed_time = SystemTime::now().duration_since(prev_time).expect("Time went backwards").as_nanos();
        let wait = if elapsed_time < 1_000_000_000u128 / 60 { 1_000_000_000u32 / 60 - (elapsed_time as u32) } else { 0 };
        ::std::thread::sleep(Duration::new(0, wait));
        prev_time = SystemTime::now();
    
    }
}