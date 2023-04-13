use std::{sync::mpsc::{channel, Sender, Receiver}, thread::{spawn, JoinHandle}, time::Duration};

use anyhow::Result;
use opencv::{videoio::{VideoCapture,self, VideoCaptureTrait,}, prelude::{Mat, MatTraitConstManual}, imgproc::{cvt_color, COLOR_BGR2RGBA}};
use opencv::prelude::VideoCaptureTraitConst;
use slint::{Timer, TimerMode, Image};
const CANVAS_WIDTH: u32 = 640;
const CANVAS_HEIGHT: u32 = 480;
const FPS:f32 = 30.0;


const CAMERA_INDEX:i32 = 0;


use slint::slint;
slint!{
    import {VerticalBox, HorizontalBox} from "std-widgets.slint";

export component Main inherits Window {
    title: "slint";
    icon: @image-url("");
    width: 810px;
    height: 650px;

    pure callback render-image(int) -> image;
    in-out property <int> frame;

    VerticalLayout { 
        HorizontalLayout {
            alignment: center;
            Rectangle {
                border-color: white;
                border-width: 1px;
                width: 800px;
                height: 640px;
                Image {
                    width: 100%;
                    height: 100%;
                    source: render-image(frame);
                }
            }
        }
    }
}

}

fn main() -> Result<()>{
    let window = Main::new().unwrap();
  
    let timer = Timer::default();
    let window_clone = window.as_weak();

    let (frame_sender, frame_receiver) = channel();
    let (exit_sender, exit_receiver) = channel();

    let mut frame_data = vec![0; (CANVAS_WIDTH * CANVAS_HEIGHT * 4) as usize];

    timer.start(TimerMode::Repeated, std::time::Duration::from_secs_f32(1./FPS), move || {
        if let Some(window) = window_clone.upgrade(){
            window.set_frame(window.get_frame()+1);
        }
    });

    let task = start(frame_sender, exit_receiver);

    let mut render = move || -> Result<Image>{

        if let Ok(frame_rgba) = frame_receiver.try_recv(){
            frame_data.copy_from_slice(&frame_rgba);
        }

        let v = slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(
            frame_data.as_slice(),
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
        ));
        Ok(v)
    };

    window.on_render_image(move |_frame|{
        render().map_err(|err| eprintln!("{:?}", err)).unwrap()
    });

    window.run().unwrap();
    println!("Closed");
    exit_sender.send(())?;
    let result = task.join().unwrap();
    println!("Camera Stopped {:?}", result);
    Ok(())
}


fn start(frame_sender: Sender<Vec<u8>>, exit_receiver: Receiver<()>) -> JoinHandle<Result<()>>{
    spawn(move || -> Result<()>{
        let mut camera = VideoCapture::new(CAMERA_INDEX, videoio::CAP_ANY)?;
        let opened = VideoCapture::is_opened(&camera)?;
        if !opened {
            panic!("Unable to open default camera!");
        }
        let mut frame_bgr = Mat::default();
        let mut frame_rgba = Mat::default();
        loop{
            if let Ok(()) = exit_receiver.try_recv(){
                break;
            }
            else{
            camera.read(&mut frame_bgr)?;

            cvt_color(&frame_bgr, &mut frame_rgba, COLOR_BGR2RGBA, 0)?;
            
            frame_sender.send(frame_rgba.data_bytes()?.to_vec())?;

            std::thread::sleep(Duration::from_millis(10));
        }
        }
        Ok(())
    })
}